#!/usr/bin/env python3
"""Completeness audit of the snapshot spec (design doc, "The completeness
audit").

Usage (from the repo root): scripts/verus-spec-audit.py [--list]

Scans every tracked .rs file with Verus code (outside verus-patches/ and
snapshot-spec-docs/) for the trust markers and TODOs, classifies them,
and checks that every
trusted, deferred, or uninterpreted function is named in
snapshot-spec-docs/TRUSTED.md (our code to prove is listed in no doc). Prints a
summary; with --list, every item. Doc matching is by name, so it finds
items the doc never mentions; an item it reports as documented may still
need a human check.

Sections:
  placeholders   assume_new(): where each is (verified code would be
                 unsound; external_body bodies, plain functions, tests are
                 not checked by Verus), and its marker, a comment just above
                 its statement: PROOF TODO: (replace when proving the body),
                 TRUSTED: (keep: a trusted function's body), WAITS FOR
                 PROTOCOL: (replace once that protocol exists), OUT OF SCOPE:
                 (keep: nothing to prove, e.g. tests, a non-verification
                 build, vmbus). Exits 1 if one is in verified code or has no
                 marker.
  functions      uninterp / external_body (with or without a contract) /
                 external; external_body with a contract is TRUSTED (the
                 trusted base, external only: a *_t.rs file, or "TRUSTED" in
                 its attributes or docs), DEFERRED (a Verus limit), or
                 PENDING (our code: a body to prove; "ASSUMED"); everything
                 in a file headed "// OUT OF SCOPE" is OUT-OF-SCOPE
  assumes        assume(..), assume_specification
  todos          TODOs: OURS (git blame: the current git user) or UPSTREAM
  lessons        the mechanical checks of snapshot-spec-docs/LESSONS.md, "How
                 to reduce these mistakes" (by pattern, so a human checks
                 each hit). Exits 1 on a FAIL check:
                 D3 FAIL   a protocol rule `=> false` in a `request_ok`
                 D3 FAIL   an `assume_new()` marked OUT OF SCOPE: outside tests
                           and non-verification builds
                 F  FAIL   a `requires` clause on a FracGhost's fraction
                           (`frac()`) with no id (`id()`), unless a comment
                           just above says `ID OK:` and why
                 F  FLAG   a protocol rule returning a half (`gout matches
                           Some(h) && h.valid(..)`) that never states its
                           value (`h@`, `gout == gin`)
                 E  FLAG   a `LoadedVm` / `InitializedVm` contract returning a
                           `Result` with no `result is Err` / `matches Err`
                 D2 FLAG   a type declared by name only (external_body
                           external_type_specification) with a specified
                           method but no `View`
"""
import importlib.util
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    'classify', os.path.join(HERE, 'verus-trust-classify.py'))
classify = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(classify)

FN_RE = re.compile(
    r'^\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?'
    r'(?:(?:const|async|unsafe|default)\s+)*'
    r'(?:(?:proof|exec)\s+|(?:open(?:\([^)]*\))?|closed)?\s*spec\s+|'
    r'(?:uninterp|broadcast)\s+(?:spec\s+)?(?:proof\s+)?)?'
    r'fn\s+([A-Za-z_0-9]+)')


def verus_files():
    out = subprocess.run(['git', 'ls-files', '*.rs'], capture_output=True,
                         text=True, check=True).stdout.split()
    files = []
    for f in out:
        if f.startswith(('verus-patches/', 'snapshot-spec-docs/')):
            continue
        try:
            text = open(f).read()
        except OSError:
            continue
        # Any file that uses Verus or vstd (placeholders can appear in a file
        # with no Verus attribute of its own).
        if 'verus_keep_ghost' in text or 'verus!' in text or 'vstd' in text:
            files.append(f)
    return files


def attrs_of(lines, i):
    return classify.full_attrs(lines, i)


def enclosing_fn(lines, i):
    """The function enclosing line i (by indentation), and its line."""
    ind = len(lines[i]) - len(lines[i].lstrip())
    for k in range(i, -1, -1):
        m = re.search(r'\bfn\s+([A-Za-z_0-9]+)', lines[k])
        if (m and not lines[k].strip().startswith('//')
                and len(lines[k]) - len(lines[k].lstrip()) < ind):
            return m.group(1), k
    return None, None


def test_lines(lines):
    """Lines inside `mod tests { .. }` blocks (by brace matching)."""
    inside = set()
    i = 0
    while i < len(lines):
        if re.match(r'\s*mod tests\s*\{', lines[i]):
            depth = 0
            j = i
            while j < len(lines):
                depth += lines[j].count('{') - lines[j].count('}')
                inside.add(j)
                if depth <= 0 and j > i:
                    break
                j += 1
            i = j + 1
        else:
            i += 1
    return inside


MARKERS = ('PROOF TODO:', 'TRUSTED:', 'WAITS FOR PROTOCOL:', 'OUT OF SCOPE:')


def marker_of(lines, i, tests, test_marked):
    """The marker in the comments just above the statement holding line i
    (or the test module's, for a test)."""
    if i in tests and test_marked:
        return 'OUT OF SCOPE:'
    for k in range(i, max(-1, i - 12), -1):
        t = lines[k].strip()
        if k != i and not t.startswith('//') and t.endswith((';', '{', '}')):
            break
        for m in MARKERS:
            if t.startswith('//') and m in t:
                return m
    return None


def placeholders(files):
    rows = []
    for f in files:
        lines = open(f).read().split('\n')
        tests = test_lines(lines)
        test_marked = any('OUT OF SCOPE:' in lines[k] for k in tests)
        for i, l in enumerate(lines):
            in_tests = i in tests
            if 'assume_new()' not in l or l.strip().startswith('//'):
                continue
            name, k = enclosing_fn(lines, i)
            near = '\n'.join(lines[max(0, i - 3):i + 1])
            if name is None:
                kind = 'NO-FN'
            else:
                a = attrs_of(lines, k)
                if in_tests:
                    kind = 'TEST'
                elif 'external_body' in a:
                    kind = 'EXTERNAL_BODY'
                elif 'verus_verify(external)' in a or 'verifier::external]' in a:
                    kind = 'EXTERNAL'
                elif 'cfg(not(verus_keep_ghost' in near:
                    kind = 'NOT-VERIFICATION-BUILD'
                elif 'verus_verify' in a or k in classify.verus_block_lines(lines):
                    kind = 'VERIFIED!'
                else:
                    kind = 'PLAIN'
            marker = marker_of(lines, i, tests, test_marked) or 'UNMARKED'
            rows.append((kind, f'{f}:{i + 1}', name or '?', marker))
    return rows


def functions(files):
    trusted_doc = open('snapshot-spec-docs/TRUSTED.md').read()
    rows = []
    for f in files:
        lines = open(f).read().split('\n')
        # A `_t.rs` file whose header says it is ASSUMED (our code) holds no
        # trusted-base items.
        header_assumed = any('ASSUMED' in h for h in lines[:15])
        # A file parked out of scope (e.g. vmbus) holds nothing to prove.
        out_of_scope = any(h.startswith('// OUT OF SCOPE') for h in lines[:15])
        for i, l in enumerate(lines):
            m = FN_RE.match(l)
            if not m:
                continue
            name = m.group(1)
            # (The docs' file name is not a label.)
            a = attrs_of(lines, i).replace('TRUSTED.md', '')
            j, sig = i, l
            while ('{' not in lines[j] and not lines[j].rstrip().endswith(';')
                   and j + 1 < len(lines)):
                j += 1
                sig += '\n' + lines[j]
            uninterp = 'uninterp' in l
            eb = 'verus_verify(external_body)' in a or 'verifier::external_body' in a
            ext = 'verus_verify(external)' in a or 'verifier::external]' in a
            if not (uninterp or eb or ext):
                continue
            contract = any(k in a or k in sig for k in ('requires', 'ensures'))
            if out_of_scope:
                kind = 'OUT-OF-SCOPE'
            elif uninterp:
                kind = 'UNINTERP'
            elif ext:
                kind = 'EXTERNAL'
            elif not contract:
                kind = 'EB-NO-CONTRACT'
            elif 'DEFERRED' in a:
                kind = 'EB-DEFERRED'
            elif 'ASSUMED' in a or header_assumed:
                kind = 'EB-PENDING'
            elif f.endswith('_t.rs') or 'TRUSTED' in a or 'Trusted' in a:
                kind = 'EB-TRUSTED'
            else:
                kind = 'EB-PENDING'
            word = re.compile(r'\b' + re.escape(name) + r'\b')
            if kind in ('EB-TRUSTED', 'EB-DEFERRED', 'UNINTERP'):
                documented = bool(word.search(trusted_doc))
            else:
                # Out of scope, or our code to prove: no doc lists them.
                documented = True
            rows.append((kind, f'{f}:{i + 1}', name, documented))
    return rows


def assumes(files):
    rows = []
    for f in files:
        for i, l in enumerate(open(f).read().split('\n')):
            s = l.strip()
            if s.startswith('//'):
                continue
            if re.search(r'\bassume\(', s):
                rows.append(('assume', f'{f}:{i + 1}', s[:90]))
            if 'assume_specification' in s:
                rows.append(('assume_specification', f'{f}:{i + 1}', s[:90]))
    return rows


def todos(files):
    """TODOs, OURS if `git blame` names the current git user as the line's
    author (the fork's main has diverged from OpenVMM's, so comparing with
    it is not reliable)."""
    me = subprocess.run(['git', 'config', 'user.name'], capture_output=True,
                        text=True).stdout.strip()
    rows = []
    for f in files:
        lines = open(f).read().split('\n')
        for i, l in enumerate(lines):
            if 'TODO' not in l or 'PROOF TODO:' in l:
                continue
            blame = subprocess.run(
                ['git', 'blame', '--porcelain', '-L', f'{i + 1},{i + 1}', '--', f],
                capture_output=True, text=True).stdout
            m = re.search(r'^author (.*)$', blame, re.M)
            author = m.group(1) if m else ''
            kind = 'OURS' if author in (me, 'Not Committed Yet') else 'UPSTREAM'
            rows.append((kind, f'{f}:{i + 1}', l.strip()[:100]))
    return rows


def clause_of(lines, i):
    """The contract clause holding line i: from the line after the previous
    clause's end (a line ending with ',', or `requires`/`ensures`) to the
    first line ending with ','."""
    a = i
    while a > 0:
        t = lines[a - 1].strip()
        if t.endswith(',') or t in ('requires', 'ensures') or t.endswith('=>'):
            break
        a -= 1
    b = i
    while b + 1 < len(lines) and not lines[b].rstrip().endswith(','):
        b += 1
    return '\n'.join(lines[a:b + 1])


def in_requires(lines, i):
    """Line i is in a `requires` clause (the nearest keyword above it)."""
    for k in range(i, max(-1, i - 40), -1):
        t = lines[k].strip()
        if re.match(r'(requires|ensures|recommends)\b', t):
            return t.startswith('requires')
    return False


def module_files(files, where):
    """The files of the module that declares a type in `where`: for
    `a/b.proof.rs` or `a/b.spec.rs`, `a/b.rs` and the files under `a/b/`."""
    path = where.split(':')[0]
    stem = re.sub(r'\.(proof|spec)\.rs$', '', path)
    stem = re.sub(r'\.rs$', '', stem)
    return [f for f in files if f == stem + '.rs' or f.startswith(stem + '/')]


def lessons(files, placeholder_rows):
    rows = []
    view_types = set()
    for f in files:
        for l in open(f).read().split('\n'):
            m = re.match(r'\s*impl(?:<[^>]*>)?\s+View\s+for\s+([A-Za-z_0-9]+)', l)
            if m:
                view_types.add(m.group(1))
    hidden = []  # (type, where)
    for f in files:
        lines = open(f).read().split('\n')
        tests = test_lines(lines)
        for i, l in enumerate(lines):
            s = l.strip()
            if s.startswith('//') or i in tests:
                continue
            name, _ = enclosing_fn(lines, i)
            # D3: `=> false` rules.
            if name == 'request_ok' and re.search(r'=>\s*false,?\s*$', s):
                rows.append(('D3 FAIL', f'{f}:{i + 1}', 'protocol rule `false`: ' + s[:60]))
            # F: a FracGhost's fraction with no id.
            if re.search(r'\bfrac\(\)\s*==', s) and in_requires(lines, i):
                c = clause_of(lines, i)
                ok = any('ID OK:' in lines[k] for k in range(max(0, i - 4), i))
                if 'id()' not in c and not ok:
                    rows.append(('F FAIL', f'{f}:{i + 1}', 'FracGhost clause without its id: ' + s[:60]))
            # F: a returned half whose value is never stated.
            if (name == 'request_ok' and re.search(r'gout matches Some\((\w+)\) && \1\.valid\(', s)):
                h = re.search(r'gout matches Some\((\w+)\)', s).group(1)
                c = clause_of(lines, i)
                if f'{h}@' not in c and 'gout == gin' not in c:
                    rows.append(('F FLAG', f'{f}:{i + 1}', 'returned half, value not stated: ' + s[:60]))
            # D2: types declared by name only.
            m = re.match(r'\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+Ex\w+\(\s*([A-Za-z_0-9]+)', l)
            if m and 'external_body' in '\n'.join(lines[max(0, i - 3):i]):
                hidden.append((m.group(1), f'{f}:{i + 1}'))
    # E: LoadedVm / InitializedVm contracts on a Result with no Err clause.
    for f in files:
        if '/worker/' not in f:
            continue
        lines = open(f).read().split('\n')
        impl_of = None
        for i, l in enumerate(lines):
            m = re.match(r'impl(?:<[^>]*>)?\s+([A-Za-z_0-9]+)\s*\{', l)
            if m:
                impl_of = m.group(1)
            elif re.match(r'(impl|mod|fn|pub|}|verus!)', l):
                impl_of = None if not l.startswith('}') else None
            mf = FN_RE.match(l)
            if not mf or impl_of not in ('LoadedVm', 'InitializedVm'):
                continue
            a = attrs_of(lines, i)
            j, sig = i, l
            while '{' not in lines[j] and j + 1 < len(lines):
                j += 1
                sig += '\n' + lines[j]
            if ('ensures' in a and 'Result' in sig
                    and not re.search(r'result (is|matches) Err', a)):
                rows.append(('E FLAG', f'{f}:{i + 1}', f'{impl_of}::{mf.group(1)}: no Err clause'))
    # D2: a hidden type with a specified method and no View.
    for ty, where in hidden:
        if ty in view_types:
            continue
        for f in module_files(files, where):
            lines = open(f).read().split('\n')
            for i, l in enumerate(lines):
                if not re.match(r'impl(?:<[^>]*>)?\s+' + re.escape(ty) + r'\b\s*(<[^>]*>)?\s*\{', l):
                    continue
                depth, j, spec = 0, i, False
                while j < len(lines):
                    depth += lines[j].count('{') - lines[j].count('}')
                    if 'verus_spec' in lines[j] or re.match(r'\s*(requires|ensures)\b', lines[j]):
                        spec = True
                    if depth <= 0 and j > i:
                        break
                    j += 1
                if spec:
                    rows.append(('D2 FLAG', where, f'{ty}: declared by name only, a method specified ({f}:{i + 1}), no View'))
                    break
            else:
                continue
            break
    # D3: OUT OF SCOPE placeholders outside tests and non-verification builds.
    for kind, where, fn, marker in placeholder_rows:
        if marker == 'OUT OF SCOPE:' and kind not in ('TEST', 'NOT-VERIFICATION-BUILD'):
            rows.append(('D3 FAIL', where, f'assume_new() marked OUT OF SCOPE in {fn}'))
    return rows


def summarize(title, rows, key=0):
    counts = {}
    for r in rows:
        counts[r[key]] = counts.get(r[key], 0) + 1
    print(f'== {title}: {len(rows)}')
    for k in sorted(counts):
        print(f'   {k:24} {counts[k]}')


def main():
    list_all = '--list' in sys.argv[1:]
    files = verus_files()
    print(f'files with Verus code: {len(files)}')

    p = placeholders(files)
    summarize('placeholders (assume_new)', p)
    summarize('placeholders by marker', p, key=3)
    bad = [r for r in p if r[0] in ('VERIFIED!', 'NO-FN') or r[3] == 'UNMARKED']
    for r in (p if list_all else bad):
        print('   ', *r, sep='\t')

    fns = functions(files)
    summarize('functions', fns)
    undoc = [r for r in fns if not r[3]]
    print(f'   not named in the docs: {len(undoc)}')
    for r in (fns if list_all else undoc):
        print('   ', *r, sep='\t')

    a = assumes(files)
    summarize('assumes', a)
    if list_all:
        for r in a:
            print('   ', *r, sep='\t')

    failed = bool(bad)
    t = todos(files)
    summarize('TODOs', t)
    for r in t:
        if list_all or r[0] == 'OURS':
            print('   ', *r, sep='\t')

    ls = lessons(files, p)
    summarize('lessons (LESSONS.md checks)', ls)
    for r in ls:
        print('   ', *r, sep='\t')
    lessons_failed = any(r[0].endswith('FAIL') for r in ls)

    if failed:
        print('FAILED: a placeholder is in verified code or has no marker '
              '(one of: ' + ', '.join(MARKERS) + ')')
    if lessons_failed:
        print('FAILED: a LESSONS.md check (D3 or F) found a hit')
    return 1 if (failed or lessons_failed) else 0


if __name__ == '__main__':
    sys.exit(main())
