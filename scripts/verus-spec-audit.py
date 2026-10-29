#!/usr/bin/env python3
"""Completeness audit of the snapshot spec (design doc, "The completeness
audit").

Usage (from the repo root): scripts/verus-spec-audit.py [--list]

Scans every tracked .rs file with Verus code (outside verus-patches/) for
the trust markers and TODOs, classifies them, and checks them against the
docs (SNAPSHOT-SPEC-TRUSTED.md, -DESIGN.md, -CODE-CHANGES.md). Prints a
summary; with --list, every item. Doc matching is by name, so it finds
items the docs never mention; an item it reports as documented may still
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

DOCS = ['SNAPSHOT-SPEC-TRUSTED.md', 'SNAPSHOT-SPEC-DESIGN.md',
        'SNAPSHOT-SPEC-CODE-CHANGES.md']

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
        if f.startswith('verus-patches/'):
            continue
        try:
            text = open(f).read()
        except OSError:
            continue
        if 'verus_keep_ghost' in text or 'verus!' in text:
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


def functions(files, docs):
    trusted_doc = open('SNAPSHOT-SPEC-TRUSTED.md').read()
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
            a = attrs_of(lines, i).replace('SNAPSHOT-SPEC-TRUSTED', '')
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
            elif kind == 'OUT-OF-SCOPE':
                documented = True
            else:
                documented = bool(word.search(docs))
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
    docs = ''.join(open(d).read() for d in DOCS)
    print(f'files with Verus code: {len(files)}')

    p = placeholders(files)
    summarize('placeholders (assume_new)', p)
    summarize('placeholders by marker', p, key=3)
    bad = [r for r in p if r[0] in ('VERIFIED!', 'NO-FN') or r[3] == 'UNMARKED']
    for r in (p if list_all else bad):
        print('   ', *r, sep='\t')

    fns = functions(files, docs)
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

    if failed:
        print('FAILED: a placeholder is in verified code or has no marker '
              '(one of: ' + ', '.join(MARKERS) + ')')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
