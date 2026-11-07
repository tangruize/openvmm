#!/usr/bin/env python3
"""Style check of Verus spec and proof code against
snapshot-spec-docs/CONVENTIONS.md.

Usage (from the repo root):
  scripts/verus-style-check.py [--rev REV] [--summary] [PATH_PREFIX ...]

Scans every tracked `.rs` file with Verus code (outside verus-patches/,
snapshot-spec-docs/, .system-proof/, tests and `*.must_fail.rs`), or only
those under the given path prefixes, and lists each place that breaks a
convention, with its file and line, so that it can be refactored. With
`--rev`, it reads the files of that git revision instead of the working
tree. With `--summary`, it prints only the counts.

Each hit is FIX (breaks a convention: refactor it) or CHECK (likely breaks
one: a human decides). A comment containing `STYLE OK:` and a reason, in
the three lines above a hit, accepts it. Exits 1 if any FIX remains.

Checks (the convention each comes from):
  free-spec-fn     FIX   a spec function at module level, not in an impl or
                         trait ("No free spec functions or free lemmas")
  free-lemma       FIX   a proof function (lemma, axiom) at module level
  spec-on-concrete FIX   a spec function other than `wf` / `<role>_wf` in an
                         inherent impl of a concrete type (one with a `View`
                         impl): it belongs to the view type ("A concrete type
                         has two spec functions: view and wf()")
  view-name        FIX   `impl View for Foo` whose `type V` is not `FooView`
                         (CHECK if it is Foo itself, a vstd collection or a
                         primitive, or a view type shared by several types)
                         ("The view of a concrete type Foo is always named
                         FooView")
  open-view        FIX   an open `view` whose body is more than a mapping of
                         fields ("Views are closed unless they are a plain
                         mapping of fields")
  large-open       CHECK an open spec function with a quantifier, a `match`,
                         or more than 12 body lines: consider closing it and
                         proving lemmas about it
  missing-wf       CHECK a concrete type (with a `View` impl) with no `wf`
  spec-in-prod     FIX   a spec or proof function defined in a production
                         `.rs` file, not in `.spec.rs`, `.proof.rs` or
                         `_t.rs` ("File layout")
  type-decl-in-prod CHECK an `external_type_specification` in a production
                         file (declarations of types to Verus go in
                         `.proof.rs`)
  long-proof-block CHECK a `proof { }` / `proof! { }` block of more than 10
                         lines in a production file: it should call lemmas
                         from `<file>.proof.rs` ("File layout")
"""
import collections
import re
import subprocess
import sys

SKIP_PREFIX = ('verus-patches/', 'snapshot-spec-docs/', '.system-proof/', 'target/')
COLLECTIONS = {'Seq', 'Map', 'Set', 'Multiset', 'Option', 'int', 'nat', 'bool',
               'u8', 'u16', 'u32', 'u64', 'u128', 'usize', 'i8', 'i16', 'i32',
               'i64', 'i128', 'isize', 'char', '()'}
FN_RE = re.compile(
    r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(open|closed|uninterp)(?:\([^)]*\))?\s+)?'
    r'(?:broadcast\s+)?(spec|proof)\s+fn\s+(\w+)')
IMPL_RE = re.compile(r'^\s*(?:unsafe\s+)?impl\b(?:\s*<[^{]*?>)?\s+(.*?)\s*(?:where\b.*)?\{?\s*$')
VIEW_TYPE_RE = re.compile(r'^\s*type\s+V\s*=\s*([^;]+);')


def files_of(rev, prefixes):
    if rev:
        names = subprocess.run(['git', 'ls-tree', '-r', '--name-only', rev],
                               capture_output=True, text=True, check=True).stdout.split()
    else:
        names = subprocess.run(['git', 'ls-files', '*.rs'], capture_output=True,
                               text=True, check=True).stdout.split()
    names = [n for n in names if n.endswith('.rs')
             and not n.startswith(SKIP_PREFIX)
             and not n.endswith('.must_fail.rs')
             and '/tests/' not in n and not n.endswith('_tests.rs')
             and (not prefixes or n.startswith(tuple(prefixes)))]
    texts = {}
    if rev:
        p = subprocess.Popen(['git', 'cat-file', '--batch'], stdin=subprocess.PIPE,
                             stdout=subprocess.PIPE)
        for n in names:
            p.stdin.write(f'{rev}:{n}\n'.encode())
            p.stdin.flush()
            header = p.stdout.readline().split()
            if len(header) < 3 or header[1] != b'blob':
                continue
            size = int(header[2])
            data = p.stdout.read(size)
            p.stdout.read(1)
            texts[n] = data.decode('utf-8', 'replace')
        p.stdin.close()
        p.wait()
    else:
        for n in names:
            try:
                texts[n] = open(n, errors='replace').read()
            except OSError:
                pass
    return {n: t for n, t in texts.items()
            if 'verus!' in t or 'verus_spec' in t or 'verus_verify' in t or 'spec fn' in t}


def code_only(line):
    """The line without its comment and string literals (roughly)."""
    line = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line)
    line = re.sub(r"'(?:[^'\\]|\\.)'", "''", line)
    i = line.find('//')
    return line if i < 0 else line[:i]


def accepted(lines, i):
    return any('STYLE OK:' in lines[k] for k in range(max(0, i - 3), i + 1))


def impl_target(text):
    """(trait, type) of an impl header, type without generics or path."""
    m = IMPL_RE.match(text)
    if not m:
        return None, None
    head = m.group(1)
    trait = None
    if ' for ' in f' {head} ':
        trait, head = re.split(r'\s+for\s+', head, maxsplit=1)
        trait = re.sub(r'<.*', '', trait).split('::')[-1].strip()
    ty = re.sub(r'<.*', '', head).strip().split('::')[-1].strip()
    return trait, ty


def scan(path, text):
    """Yields (line index, kind, container) for each spec/proof fn, and
    records impls, views and proof blocks."""
    lines = text.split('\n')
    stack = []  # entries: ('impl', trait, type) | ('trait',) | ('mod',) | ('fn',) | ('block',)
    fns, views, proof_blocks = [], [], []
    pending = ''
    proof_start = None
    proof_depth = None
    for i, raw in enumerate(lines):
        line = code_only(raw)
        s = line.strip()
        header = (pending + ' ' + s).strip() if pending else s
        m = FN_RE.match(line)
        if m and not raw.strip().startswith('//'):
            container = next((e for e in reversed(stack) if e[0] in ('impl', 'trait', 'mod', 'fn')), ('mod',))
            fns.append((i, m.group(1) or 'plain', m.group(2), m.group(3), container))
        vm = VIEW_TYPE_RE.match(line)
        if vm:
            impl = next((e for e in reversed(stack) if e[0] == 'impl'), None)
            if impl and impl[1] == 'View':
                views.append((i, impl[2], vm.group(1).strip()))
        if re.match(r'(proof!?\s*\{|proof!\s*\{)', s) and proof_start is None:
            proof_start, proof_depth = i, len(stack)
        for ch in line:
            if ch == '{':
                if re.match(r'^(?:unsafe\s+)?impl\b', header):
                    trait, ty = impl_target(header)
                    stack.append(('impl', trait, ty))
                elif re.match(r'^(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait\b', header):
                    stack.append(('trait',))
                elif re.match(r'^(?:pub(?:\([^)]*\))?\s+)?mod\b', header) or header.startswith(('verus!', '::vstd::prelude::verus!', 'vstd::prelude::verus!')):
                    stack.append(('mod',))
                elif re.search(r'\bfn\s+\w+', header):
                    stack.append(('fn',))
                else:
                    stack.append(('block',))
                header = ''
            elif ch == '}':
                if stack:
                    stack.pop()
                if proof_start is not None and len(stack) <= proof_depth:
                    proof_blocks.append((proof_start, i))
                    proof_start = None
        # An item header may span lines (e.g. `impl<T> Foo<T>` then `where ...` then `{`).
        if '{' in line:
            pending = ''
        elif pending or re.match(r'^(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(impl|trait|mod)\b', s):
            pending = header
        else:
            pending = ''
    return lines, fns, views, proof_blocks


def body_of(lines, i):
    """The lines of the function starting at line i, up to its closing brace."""
    depth, started, out = 0, False, []
    for k in range(i, min(len(lines), i + 400)):
        c = code_only(lines[k])
        out.append(c)
        depth += c.count('{') - c.count('}')
        if '{' in c:
            started = True
        if started and depth <= 0:
            break
        if not started and c.rstrip().endswith(';'):
            break
    return out


def main():
    args = sys.argv[1:]
    rev = None
    if '--rev' in args:
        k = args.index('--rev')
        rev = args[k + 1]
        del args[k:k + 2]
    summary = '--summary' in args
    prefixes = [a for a in args if not a.startswith('--')]
    texts = files_of(rev, prefixes)

    hits = []
    counts = collections.Counter()
    concrete = {}      # type -> (file, line) of its View impl
    view_types = set()
    view_users = collections.Counter()  # view type -> number of types viewed as it
    scanned = {}
    for path, text in texts.items():
        lines, fns, views, proof_blocks = scan(path, text)
        scanned[path] = (lines, fns, views, proof_blocks)
        for i, ty, v in views:
            concrete.setdefault(ty, (path, i))
            vn = re.sub(r'<.*', '', v).split('::')[-1].strip()
            view_types.add(vn)
            view_users[vn] += 1

    wf_types = set()
    for path, (lines, fns, views, proof_blocks) in scanned.items():
        for i, mode, kind, name, container in fns:
            if kind == 'spec' and (name == 'wf' or name.endswith('_wf')) and container[0] == 'impl':
                wf_types.add(container[2])

    for path, (lines, fns, views, proof_blocks) in scanned.items():
        prod = not path.endswith(('.spec.rs', '.proof.rs', '_t.rs'))
        counts['files'] += 1
        for i, mode, kind, name, container in fns:
            counts[f'{kind} fns'] += 1
            if kind == 'spec':
                counts[f'spec fns {mode}'] += 1
            where = f'{path}:{i + 1}'
            ok = accepted(lines, i)
            free = container[0] in ('mod',)
            if free:
                counts[f'free {kind} fns'] += 1
                if not ok:
                    hits.append(('FIX', 'free-spec-fn' if kind == 'spec' else 'free-lemma', where,
                                 f'{kind} fn {name} is free: put it on the type it is about'))
            if (kind == 'spec' and container[0] == 'impl' and container[1] is None
                    and container[2] in concrete and container[2] not in view_types
                    and name != 'view' and name != 'wf' and not name.endswith('_wf') and not ok):
                hits.append(('FIX', 'spec-on-concrete', where,
                             f'spec fn {name} on concrete type {container[2]}: put it on its view type'))
            if prod and not ok:
                hits.append(('FIX', 'spec-in-prod', where,
                             f'{kind} fn {name} in a production file: move it to '
                             f'{"the .spec.rs" if kind == "spec" else "the .proof.rs"}'))
            if kind == 'spec' and mode == 'open':
                body = body_of(lines, i)
                text_b = '\n'.join(body[1:])
                if name == 'view' and container[0] == 'impl' and container[1] == 'View':
                    if re.search(r'\b(if|match|forall|exists|let)\b', text_b) and not ok:
                        hits.append(('FIX', 'open-view', where,
                                     f'open view of {container[2]} is more than a mapping of fields: close it'))
                elif (re.search(r'\b(forall|exists)\s*\|', text_b) or re.search(r'\bmatch\b', text_b)
                      or len(body) > 14) and not ok:
                    hits.append(('CHECK', 'large-open', where,
                                 f'open spec fn {name} ({len(body)} lines'
                                 f'{", quantifier" if re.search(r"(forall|exists)\s*[|]", text_b) else ""}'
                                 f'{", match" if re.search(r"[m]atch", text_b) else ""}): consider closing it'))
        for i, ty, v in views:
            counts['View impls'] += 1
            vname = re.sub(r'<.*', '', v).split('::')[-1].strip()
            if '$' in ty or '$' in vname:
                continue
            if vname != ty + 'View' and not accepted(lines, i):
                if vname == ty:
                    sev, why = 'CHECK', 'the type itself (no view type)'
                elif vname in COLLECTIONS:
                    sev, why = 'CHECK', 'a vstd collection or primitive'
                elif view_users[vname] > 1:
                    sev, why = 'CHECK', f'shared with {view_users[vname] - 1} other type(s)'
                else:
                    sev, why = 'FIX', 'misnamed'
                hits.append((sev, 'view-name', f'{path}:{i + 1}',
                             f'the view of {ty} is {v}, not {ty}View: {why}'))
        if prod:
            for k, l in enumerate(lines):
                if 'external_type_specification' in l and not l.strip().startswith('//') and not accepted(lines, k):
                    hits.append(('CHECK', 'type-decl-in-prod', f'{path}:{k + 1}',
                                 'a type declared to Verus in a production file: move it to the .proof.rs'))
            for a, b in proof_blocks:
                n = b - a + 1
                counts['proof blocks in production files'] += 1
                if n > 10 and not accepted(lines, a):
                    hits.append(('CHECK', 'long-proof-block', f'{path}:{a + 1}',
                                 f'a proof block of {n} lines in a production file: '
                                 'move its reasoning into a lemma in the .proof.rs'))
    for ty, (path, i) in concrete.items():
        if ty not in wf_types and ty not in view_types:
            hits.append(('CHECK', 'missing-wf', f'{path}:{i + 1}', f'concrete type {ty} has a View but no wf'))

    by_check = collections.Counter((h[0], h[1]) for h in hits)
    if not summary:
        for h in sorted(hits, key=lambda h: (h[1], h[2])):
            print('\t'.join(h))
    print('== counts')
    for k in sorted(counts):
        print(f'   {k:36} {counts[k]}')
    print('== hits')
    for (sev, check), n in sorted(by_check.items()):
        print(f'   {sev:5} {check:20} {n}')
    fixes = sum(n for (sev, _), n in by_check.items() if sev == 'FIX')
    if fixes:
        print(f'FAILED: {fixes} FIX hits (see snapshot-spec-docs/CONVENTIONS.md)')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
