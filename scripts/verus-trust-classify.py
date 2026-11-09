#!/usr/bin/env python3
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Classify functions/impls by their Verus status, for the trust audit
(snapshot-spec-docs/DESIGN.md, "The trust audit").

A plain-Rust impl of a trait that has a Verus contract is assumed to meet
it, silently; this script finds such code along a path. It is a
line-based heuristic (attributes above an item, `verus! {}` blocks, the
enclosing `impl`), not a parser: check surprising results by hand.

Usage: scripts/verus-trust-classify.py FILE PATTERN [PATTERN ...]
Example:
  scripts/verus-trust-classify.py vm/chipset_device_resources/src/lib.rs \
      'impl ProtobufSaveRestore for'

Each PATTERN is a regex matched against lines (e.g. 'fn save\\b' or
'impl .*SaveRestore for'). For each match, prints line, status, and text.

Status:
  VERIFIED      inside verus! {} without external_body, or verus_verify
                without external_body / external (body is proved)
  CONTRACT      verus_verify(external_body) or #[verifier::external_body]
                (contract stated, body not proved)
  EXTERNAL      verus_verify(external) / #[verifier::external]
  IMPL-VERUS    the enclosing impl block is verus_verify (method inherits)
  PLAIN         plain Rust (not seen by Verus)
"""
import re, sys

def verus_block_lines(lines):
    """Set of line indexes inside `verus! { ... }` blocks."""
    inside = set()
    i = 0
    n = len(lines)
    while i < n:
        if re.search(r'\bverus!\s*\{', lines[i]):
            depth = 0
            j = i
            started = False
            while j < n:
                for ch in lines[j]:
                    if ch == '{':
                        depth += 1
                        started = True
                    elif ch == '}':
                        depth -= 1
                inside.add(j)
                if started and depth <= 0:
                    break
                j += 1
            i = j + 1
        else:
            i += 1
    return inside

def attrs_before(lines, idx):
    """Attribute and doc lines directly above line idx."""
    out = []
    k = idx - 1
    depth = 0
    while k >= 0:
        l = lines[k].strip()
        if l.startswith('#[') or l.startswith('///') or l.startswith('//') or depth > 0 \
                or l.endswith('))]') or l.startswith('requires') or l.startswith('ensures') \
                or l.startswith(')') or l == '' and False:
            out.append(l)
            k -= 1
            continue
        # multi-line attribute bodies (verus_spec contents)
        if re.match(r'^[a-z_(].*[,)]$', l) and out and any('verus_spec' in a for a in out) is False:
            out.append(l)
            k -= 1
            continue
        break
    return '\n'.join(reversed(out)), k

def enclosing_impl(lines, idx):
    """Find the enclosing `impl` line (by indentation) and its attributes."""
    indent = len(lines[idx]) - len(lines[idx].lstrip())
    for k in range(idx - 1, -1, -1):
        l = lines[k]
        if l.strip() == '':
            continue
        ind = len(l) - len(l.lstrip())
        if ind < indent and re.match(r'\s*(pub(\([^)]*\))?\s+)?(unsafe\s+)?impl\b', l):
            a, _ = attrs_before(lines, k)
            return l.strip(), a
        if ind < indent and re.match(r'\s*(pub(\([^)]*\))?\s+)?(mod|trait|fn)\b', l):
            return None, ''
    return None, ''

def full_attrs(lines, idx):
    """The attribute/doc lines directly above line idx, following bracket
    balance so that multi-line attributes are included whole."""
    k = idx - 1
    chunk = []
    bal = 0  # open brackets still to be matched, scanning upward
    while k >= 0:
        l = lines[k].strip()
        opens = l.count('(') + l.count('[') + l.count('{')
        closes = l.count(')') + l.count(']') + l.count('}')
        if bal == 0 and not (l.startswith('#[') or l.startswith('///') or l.startswith('//')):
            # inside nothing: only a closing line of a multi-line attribute
            # (e.g. `))]`) continues the block
            if closes > opens and l.endswith(']'):
                pass
            else:
                break
        bal += closes - opens
        chunk.append(l)
        k -= 1
    return '\n'.join(reversed(chunk))

def classify(path, pats):
    lines = open(path).read().split('\n')
    vb = verus_block_lines(lines)
    for i, l in enumerate(lines):
        for p in pats:
            if re.search(p, l):
                a = full_attrs(lines, i)
                if 'verus_verify(external_body)' in a or 'verifier::external_body' in a:
                    st = 'CONTRACT'
                elif 'verus_verify(external)' in a or 'verifier::external]' in a:
                    st = 'EXTERNAL'
                elif 'verus_verify' in a:
                    st = 'VERIFIED'
                elif i in vb:
                    st = 'VERIFIED(verus!)' if 'external_body' not in a else 'CONTRACT'
                else:
                    imp, ia = enclosing_impl(lines, i)
                    if imp and 'verus_verify' in ia:
                        st = 'IMPL-VERUS'
                    else:
                        st = 'PLAIN'
                print(f'{path}:{i+1}: {st:16} {l.strip()[:90]}')

if __name__ == '__main__':
    classify(sys.argv[1], sys.argv[2:])
