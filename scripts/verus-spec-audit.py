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
                 not checked by Verus)
  functions      uninterp / external_body (with or without a contract) /
                 external; external_body with a contract is TRUSTED (a
                 boundary: a *_t.rs file, or "TRUSTED"/"Trusted" in its
                 attributes or docs) or PENDING (a body to prove)
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


def placeholders(files):
    rows = []
    for f in files:
        lines = open(f).read().split('\n')
        in_tests = False
        for i, l in enumerate(lines):
            if re.match(r'\s*mod tests\b', l):
                in_tests = True
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
            rows.append((kind, f'{f}:{i + 1}', name or '?'))
    return rows


def functions(files, docs):
    trusted_doc = open('SNAPSHOT-SPEC-TRUSTED.md').read()
    rows = []
    for f in files:
        lines = open(f).read().split('\n')
        for i, l in enumerate(lines):
            m = FN_RE.match(l)
            if not m:
                continue
            name = m.group(1)
            a = attrs_of(lines, i)
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
            if uninterp:
                kind = 'UNINTERP'
            elif ext:
                kind = 'EXTERNAL'
            elif not contract:
                kind = 'EB-NO-CONTRACT'
            elif f.endswith('_t.rs') or 'TRUSTED' in a or 'Trusted' in a:
                kind = 'EB-TRUSTED'
            else:
                kind = 'EB-PENDING'
            word = re.compile(r'\b' + re.escape(name) + r'\b')
            if kind in ('EB-TRUSTED', 'UNINTERP'):
                documented = bool(word.search(trusted_doc))
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
            if 'TODO' not in l:
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
    bad = [r for r in p if r[0] in ('VERIFIED!', 'NO-FN')]
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

    t = todos(files)
    summarize('TODOs', t)
    for r in t:
        if list_all or r[0] == 'OURS':
            print('   ', *r, sep='\t')


if __name__ == '__main__':
    main()
