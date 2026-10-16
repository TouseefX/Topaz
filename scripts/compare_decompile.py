#!/usr/bin/env python3
"""Compares a decompiled Luau file with a reference Luau file.

Renaming does not change what a script does, but it does change local
names, so the comparison looks at what names cannot hide:

* string and number constants in every function (remote names, field names,
  literal values, error messages);
* the number of functions and the count of each bytecode operation, from
  `luau-compile -g0 -O0`;
* functions that have the same normalised code (duplicated code);
* the `luau-analyze` errors, grouped by kind, with the names of unknown
  globals listed (an unknown global is a variable that is used but never
  declared, so it is nil at run time).

Passing these checks does not prove the two scripts behave the same. It finds
the differences that would make one of them break.

usage: compare_decompile.py REFERENCE.luau OUTPUT.luau LUAU_COMPILE LUAU_ANALYZE
"""

import collections
import hashlib
import re
import subprocess
import sys


def run(cmd):
    proc = subprocess.run(cmd, capture_output=True, text=True)
    return proc.stdout + proc.stderr


def compiled(path, compiler):
    """Text listing with constants, from luau-compile -g0 -O0."""
    return run([compiler, "--text", "-g0", "-O0", "--dump-constants", path])


def constants(listing):
    strings = collections.Counter()
    numbers = collections.Counter()
    for line in listing.splitlines():
        m = re.match(r"^K\d+: '(.*)'$", line)
        if m:
            strings[m.group(1)] += 1
            continue
        m = re.match(r"^K\d+: (-?[0-9][^ ]*|nan|-nan|inf|-inf)$", line)
        if m:
            numbers[m.group(1)] += 1
    return strings, numbers


def instructions(listing):
    for line in listing.splitlines():
        m = re.match(r"^([A-Z][A-Z0-9_]+) ", line)
        if m and m.group(1) != "REMARK":
            yield m.group(1), line


def opcodes(listing):
    counts = collections.Counter(op for op, _ in instructions(listing))
    return counts


def functions(listing):
    """Each function's instructions, with registers and constant slots removed,
    so identical code compares equal whatever the local names are."""
    bodies = []
    current = None
    for line in listing.splitlines():
        if re.match(r"^Function \d+", line):
            if current is not None:
                bodies.append(current)
            current = []
            continue
        if current is None:
            continue
        m = re.match(r"^([A-Z][A-Z0-9_]+) ", line)
        if not m or m.group(1) == "REMARK":
            continue
        # Drop register, upvalue and constant slot numbers; keep the value.
        text = re.sub(r"\b[RKU]\d+\b", "", line)
        text = re.sub(r"\s+", " ", text).strip()
        current.append(text)
    if current is not None:
        bodies.append(current)
    return bodies


def duplicate_report(listing, label, limit=12):
    bodies = [b for b in functions(listing) if len(b) >= 4]
    sig_count = collections.Counter(
        hashlib.sha1("\n".join(b).encode()).hexdigest() for b in bodies
    )
    by_sig = {}
    for b in bodies:
        by_sig.setdefault(hashlib.sha1("\n".join(b).encode()).hexdigest(), b)
    dup_functions = sum(c - 1 for c in sig_count.values() if c > 1)
    lines = [
        f"{label}: {len(bodies)} functions with 4+ instructions, "
        f"{len(sig_count)} distinct shapes, {dup_functions} repeats of an existing shape"
    ]
    for sig, count in sig_count.most_common(limit):
        if count < 2:
            break
        body = by_sig[sig]
        first = body[0][:90]
        lines.append(f"  {count}x shape of {len(body)} instructions, starts: {first}")
    return lines


def shape_diff(ref_listing, out_listing, limit=12):
    """Functions whose normalised code has no counterpart in the other file."""
    def shapes(listing):
        out = {}
        for body in functions(listing):
            if len(body) < 4:
                continue
            key = hashlib.sha1("\n".join(body).encode()).hexdigest()
            out.setdefault(key, body)
        return out

    def count(listing):
        return collections.Counter(
            hashlib.sha1("\n".join(b).encode()).hexdigest()
            for b in functions(listing)
            if len(b) >= 4
        )

    ref_count = count(ref_listing)
    out_count = count(out_listing)
    ref_bodies = shapes(ref_listing)
    out_bodies = shapes(out_listing)
    lines = []
    for label, mine, other, bodies in (
        ("output", out_count, ref_count, out_bodies),
        ("reference", ref_count, out_count, ref_bodies),
    ):
        missing = [k for k in mine if k not in other]
        total = sum(mine[k] for k in missing)
        lines.append(
            f"{label} functions whose shape is not in the other file: "
            f"{total} functions, {len(missing)} shapes"
        )
        missing.sort(key=lambda k: -len(bodies[k]))
        for k in missing[:limit]:
            body = bodies[k]
            strings = []
            for line in body:
                for v in re.findall(r"\['([^']*)'\]", line):
                    if v not in strings:
                        strings.append(v)
            lines.append(
                f"  {mine[k]}x, {len(body)} instructions, strings: "
                + ", ".join(repr(v) for v in strings[:8])
            )
    return lines


def analyze_kinds(path, analyzer):
    text = run([analyzer, path])
    kinds = collections.Counter()
    unknown_globals = collections.Counter()
    for line in text.splitlines():
        m = re.match(r"^.*\(\d+,\d+\): (\w+): (.*)$", line)
        if not m:
            continue
        message = m.group(2)
        g = re.match(r"^Unknown global '([^']*)'", message)
        if g:
            unknown_globals[g.group(1)] += 1
        # Ids and numbers change between the two files, so they are removed.
        message = re.sub(r"\*blocked-\d+\*", "*blocked*", message)
        message = re.sub(r"\d+", "N", message)
        kind = m.group(1) + ": " + re.sub(r"'[^']*'", "'X'", message)
        kinds[kind] += 1
    return kinds, unknown_globals


ROBLOX_GLOBALS = {
    "game", "workspace", "task", "CFrame", "Enum", "Vector3", "Vector2", "Color3",
    "UDim2", "UDim", "Instance", "Random", "NumberRange", "NumberSequence",
    "ColorSequence", "Ray", "Region3", "TweenInfo", "Rect", "BrickColor", "script",
    "shared", "plugin", "tick", "warn", "wait", "spawn", "delay", "vector",
    "DateTime", "buffer", "Font", "PhysicalProperties", "NumberSequenceKeypoint",
}


def unknown_positions(path, analyzer):
    """(line, name) for each `Unknown global` the analyzer reports."""
    text = run([analyzer, path])
    found = []
    for line in text.splitlines():
        m = re.match(r"^.*\((\d+),\d+\): TypeError: Unknown global '([^']*)'", line)
        if m:
            found.append((int(m.group(1)), m.group(2)))
    return found


def undeclared_report(output_path, positions, ref_unknown, out_unknown, limit=160):
    """Names that the output uses more often than the reference, and that are
    not Roblox globals: each is a variable used where nothing declares it."""
    with open(output_path, encoding="utf-8", errors="replace") as fh:
        lines = fh.read().split("\n")
    suspects = [
        name
        for name, count in out_unknown.items()
        if count > ref_unknown.get(name, 0) and name not in ROBLOX_GLOBALS
    ]
    absolute = {n: c for n, c in out_unknown.items() if n not in ROBLOX_GLOBALS}
    ref_absolute = {n: c for n, c in ref_unknown.items() if n not in ROBLOX_GLOBALS}
    lines_out = [
        f"absolute, not Roblox globals: output {len(absolute)} names / "
        f"{sum(absolute.values())} uses, reference {len(ref_absolute)} names / "
        f"{sum(ref_absolute.values())} uses",
        f"unknown names used more in the output than in the reference, not Roblox "
        f"globals: {len(suspects)} names, {sum(out_unknown[n] - ref_unknown.get(n, 0) for n in suspects)} uses"
    ]
    if not suspects:
        return lines_out
    suspect_set = set(suspects)
    shown = 0
    for line_no, name in sorted(positions):
        if name not in suspect_set:
            continue
        if shown >= limit:
            lines_out.append("  ... more not shown")
            break
        shown += 1
        text = lines[line_no - 1].strip() if 0 < line_no <= len(lines) else "?"
        header = "?"
        for back in range(line_no - 1, max(-1, line_no - 400), -1):
            if re.search(r"\bfunction\b", lines[back]):
                header = lines[back].strip()[:90]
                break
        declared = sum(
            1 for l in lines if re.search(r"\blocal\b", l) and re.search(r"\b" + re.escape(name) + r"\b", l)
        )
        lines_out.append(
            f"  L{line_no} {name} (declared on {declared} lines): {text[:150]}"
        )
        lines_out.append(f"      in: {header}")
    return lines_out


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

    ref_listing = compiled(reference, compiler)
    out_listing = compiled(output, compiler)
    ref_strings, ref_numbers = constants(ref_listing)
    out_strings, out_numbers = constants(out_listing)

    report = []
    ref_fn = sum(1 for line in ref_listing.splitlines() if re.match(r"^Function \d+", line))
    out_fn = sum(1 for line in out_listing.splitlines() if re.match(r"^Function \d+", line))
    report.append(f"functions: reference {ref_fn}, output {out_fn}")

    ref_ops = opcodes(ref_listing)
    out_ops = opcodes(out_listing)
    report.append("opcodes (reference -> output, changed only):")
    for op in sorted(set(ref_ops) | set(out_ops), key=lambda o: -max(ref_ops[o], out_ops[o])):
        if ref_ops[op] != out_ops[op]:
            report.append(f"  {op}: {ref_ops[op]} -> {out_ops[op]}")

    report.extend(diff_counter("string constants", ref_strings, out_strings))
    report.extend(diff_counter("number constants", ref_numbers, out_numbers))
    report.extend(duplicate_report(ref_listing, "reference duplicates"))
    report.extend(duplicate_report(out_listing, "output duplicates"))
    report.extend(shape_diff(ref_listing, out_listing))

    ref_kinds, ref_unknown = analyze_kinds(reference, analyzer)
    out_kinds, out_unknown = analyze_kinds(output, analyzer)
    report.extend(diff_counter("analyzer errors by kind", ref_kinds, out_kinds, limit=60))
    report.extend(diff_counter("unknown globals", ref_unknown, out_unknown, limit=80))
    report.extend(undeclared_report(output, unknown_positions(output, analyzer), ref_unknown, out_unknown))
    with open(output, encoding="utf-8", errors="replace") as fh:
        scope_lines = [l.strip() for l in fh if l.strip().startswith("-- TOPAZ_SCOPE")]
    report.append(f"topaz scope check lines in output: {len(scope_lines)}")
    out_by_stage = collections.Counter()
    never_by_stage = collections.Counter()
    ld_totals = [0, 0, 0]
    pl_totals = [0, 0]
    nonzero = []
    for l in scope_lines:
        if "TOPAZ_SCOPE function line" not in l:
            continue
        hit = False
        for stage, unresolved, never, skipped in re.findall(
            r"([a-z_]+)=(\d+)/(\d+)/(\d+)", l.split(":", 1)[1]
        ):
            if stage == "pl":
                pl_totals[0] += int(unresolved)
                pl_totals[1] += int(never)
                continue
            if stage == "ld":
                # LocalDeclarer: blocks not locked, declarations dropped, read-never-written locals.
                ld_totals[0] += int(unresolved)
                ld_totals[1] += int(never)
                ld_totals[2] += int(skipped)
                continue
            out_by_stage[stage] += int(unresolved) - int(never)
            never_by_stage[stage] += int(never)
            hit = hit or int(unresolved) > int(never)
        if hit and len(nonzero) < 40:
            nonzero.append(l[:300])
    report.append(
        "reads of a local out of its scope, summed over all functions, by stage: "
        + ", ".join(f"{k} {v}" for k, v in out_by_stage.items())
    )
    report.append(
        "LocalDeclarer silent skips, summed over functions: blocks not locked "
        f"{ld_totals[0]}, declarations dropped with their block {ld_totals[1]}, "
        f"locals read but never written {ld_totals[2]}"
    )
    report.append(
        "LocalDeclarer placements: declaration point not covering every use "
        f"{pl_totals[0]}, placements with no block {pl_totals[1]}"
    )
    report.append(
        "reads of a local never declared in the function (upvalue identities, noise before link): "
        + ", ".join(f"{k} {v}" for k, v in never_by_stage.items())
    )
    # Out-of-scope reads first, so they fit in the annotation budget.
    oos = [l for l in scope_lines if "(out of scope" in l]
    report.append(f"out-of-scope examples recorded: {len(oos)} (first 40 shown)")
    report.extend(l[:200] for l in oos[:40])
    stmts = [l for l in scope_lines if " oos stmt: " in l and "declare" in l]
    report.append(f"declare-stage statements with an out-of-scope read: {len(stmts)} (first 25 shown)")
    report.extend(l[:200] for l in stmts[:25])
    report.extend(nonzero[:20])
    report.extend(scope_lines[:200])

    print("\n".join(report))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
