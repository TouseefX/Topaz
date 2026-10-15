#!/usr/bin/env python3
"""Compares a decompiled Luau file with a reference Luau file.

Renaming does not change what a script does, but it does change local
names, so the comparison looks at what names cannot hide:

* string and number constants in every function (remote names, field names,
  literal values, error messages);
* the number of functions and the count of each bytecode operation, from
  `luau-compile -g0 -O0`;
* the `luau-analyze` errors, grouped by kind (unknown symbols, wrong arity,
  type errors), with names removed.

Passing these checks does not prove the two scripts behave the same. It finds
the differences that would make one of them break.

usage: compare_decompile.py REFERENCE.luau OUTPUT.luau LUAU_COMPILE LUAU_ANALYZE
"""

import collections
import re
import subprocess
import sys


def run(cmd):
    proc = subprocess.run(cmd, capture_output=True, text=True)
    return proc.stdout + proc.stderr


def constants(path, compiler):
    text = run([compiler, "--text", "-g0", "-O0", "--dump-constants", path])
    strings = collections.Counter()
    numbers = collections.Counter()
    for line in text.splitlines():
        m = re.match(r"^K\d+: '(.*)'$", line)
        if m:
            strings[m.group(1)] += 1
            continue
        m = re.match(r"^K\d+: (-?[0-9][^ ]*|nan|-nan|inf|-inf)$", line)
        if m:
            numbers[m.group(1)] += 1
    return strings, numbers, text


def opcodes(listing):
    counts = collections.Counter()
    for line in listing.splitlines():
        m = re.match(r"^([A-Z][A-Z0-9_]+) ", line)
        if m and m.group(1) != "REMARK":
            counts[m.group(1)] += 1
    return counts


def function_count(listing):
    return sum(1 for line in listing.splitlines() if re.match(r"^Function \d+", line))


def analyze_kinds(path, analyzer):
    text = run([analyzer, path])
    kinds = collections.Counter()
    for line in text.splitlines():
        m = re.match(r"^.*\(\d+,\d+\): (\w+): (.*)$", line)
        if not m:
            continue
        # Names change between the two files, so they are removed from the kind.
        kind = m.group(1) + ": " + re.sub(r"'[^']*'", "'X'", m.group(2))
        kinds[kind] += 1
    return kinds, text


def diff_counter(name, ref, out, limit=40):
    lines = []
    only_ref = ref - out
    only_out = out - ref
    lines.append(f"{name}: reference {sum(ref.values())}, output {sum(out.values())}")
    lines.append(f"  only in reference: {len(only_ref)} distinct, {sum(only_ref.values())} total")
    for key, count in sorted(only_ref.items(), key=lambda kv: -kv[1])[:limit]:
        lines.append(f"    - {count}x {key!r}")
    lines.append(f"  only in output: {len(only_out)} distinct, {sum(only_out.values())} total")
    for key, count in sorted(only_out.items(), key=lambda kv: -kv[1])[:limit]:
        lines.append(f"    + {count}x {key!r}")
    return lines


def main(argv):
    if len(argv) != 5:
        print(__doc__)
        return 2
    reference, output, compiler, analyzer = argv[1:]

    ref_strings, ref_numbers, ref_listing = constants(reference, compiler)
    out_strings, out_numbers, out_listing = constants(output, compiler)

    report = []
    report.append(
        f"functions: reference {function_count(ref_listing)}, output {function_count(out_listing)}"
    )

    ref_ops = opcodes(ref_listing)
    out_ops = opcodes(out_listing)
    report.append("opcodes (reference -> output, changed only):")
    for op in sorted(set(ref_ops) | set(out_ops), key=lambda o: -max(ref_ops[o], out_ops[o])):
        if ref_ops[op] != out_ops[op]:
            report.append(f"  {op}: {ref_ops[op]} -> {out_ops[op]}")

    report.extend(diff_counter("string constants", ref_strings, out_strings))
    report.extend(diff_counter("number constants", ref_numbers, out_numbers))

    ref_kinds, ref_analysis = analyze_kinds(reference, analyzer)
    out_kinds, out_analysis = analyze_kinds(output, analyzer)
    report.extend(diff_counter("analyzer errors by kind", ref_kinds, out_kinds, limit=60))

    print("\n".join(report))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
