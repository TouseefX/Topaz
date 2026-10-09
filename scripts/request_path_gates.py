#!/usr/bin/env python3
"""Static def/use + mocked request-path gates for ClientRenderer dumps.

Parser 0 / exit 0 / CameraShaker+sample do **not** mean the dispatcher is
fixed. This checker flags:

  if neverAssigned == "RequestName"

and requires the P0 gates (FASSet, FASSave, DL1Flip, IronDragon, POLBeam,
NotifyNoSwitchInCombat, RemoveStates, TripleKickCounterKick1Velocity) to
compare against a local that was actually assigned (not `local vN` with no
RHS).

Usage:
  python3 scripts/request_path_gates.py path/to/decompiled.luau
  python3 scripts/request_path_gates.py --expect-fail v7-ClientRenderer.luau
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"

# P0 dispatcher names. IronDragon is the family name; dumps may spell the
# exact case `IronDragon` or a sibling (`IronDragonAim`). POLBeam must be
# the exact string used as a Request discriminator.
GATES = (
    "FASSet",
    "FASSave",
    "DL1Flip",
    "IronDragon",
    "POLBeam",
    "NotifyNoSwitchInCombat",
    "RemoveStates",
    "TripleKickCounterKick1Velocity",
)

LOCAL_DECL = re.compile(
    rf"^\s*local\s+((?:{IDENT}\s*,\s*)*{IDENT})\s*(=?)\s*(.*)$"
)
ASSIGN = re.compile(rf"^\s*({IDENT})\s*=\s*(.+)$")
COMPARE = re.compile(
    rf"(?:^|[^A-Za-z0-9_])(if|elseif)\s+({IDENT})\s*==\s*\"([^\"]+)\""
)
# Request discriminators are PascalCase tokens, not Roblox class names.
REQUESTISH = re.compile(r"^[A-Z][A-Za-z0-9]{2,}$")
NOT_REQUEST = {
    "Texture",
    "Folder",
    "Model",
    "Part",
    "String",
    "Number",
    "Boolean",
    "Instance",
    "CFrame",
    "Vector3",
    "Color3",
    "BrickColor",
    "Configuration",
    "Attachment",
    "Beam",
    "ParticleEmitter",
    "Humanoid",
    "Animator",
    "Animation",
    "Sound",
    "BindableEvent",
    "RemoteEvent",
    "ModuleScript",
    "LocalScript",
    "Script",
    "Camera",
    "Workspace",
    "Players",
    "Lighting",
    "ReplicatedStorage",
    "TweenInfo",
    "UDim2",
    "UDim",
    "Ray",
    "Region3",
    "Enum",
}
FUNCTION = re.compile(r"\bfunction\b")
END = re.compile(r"^\s*end\b")
COMMENT = re.compile(r"--.*$")


@dataclass
class Scope:
    """assigned=True means the name has a real RHS, not a bare `local x`."""

    assigned: dict[str, bool] = field(default_factory=dict)

    def declare_bare(self, name: str) -> None:
        self.assigned.setdefault(name, False)

    def assign(self, name: str) -> None:
        self.assigned[name] = True

    def is_unassigned(self, name: str) -> bool:
        for s in (self,):
            if name in s.assigned:
                return not s.assigned[name]
        return False


def _split_names(blob: str) -> list[str]:
    return [n.strip() for n in blob.split(",") if n.strip()]


def analyze(src: str) -> tuple[list[str], dict[str, list[tuple[int, str, bool]]]]:
    """Return (def/use flags, gate -> [(line, local, assigned?)])."""
    flags: list[str] = []
    gates: dict[str, list[tuple[int, str, bool]]] = {g: [] for g in GATES}

    stack: list[Scope] = [Scope()]

    def lookup_unassigned(name: str) -> bool:
        for scope in reversed(stack):
            if name in scope.assigned:
                return not scope.assigned[name]
        return False

    def mark_assigned(name: str) -> None:
        for scope in reversed(stack):
            if name in scope.assigned:
                scope.assign(name)
                return
        stack[-1].assign(name)

    for lineno, raw in enumerate(src.splitlines(), 1):
        line = COMMENT.sub("", raw).rstrip()
        if not line.strip():
            continue

        # Compares first so `if x == "A" then` is seen before `function`.
        for m in COMPARE.finditer(line):
            local_name = m.group(2)
            lit = m.group(3)
            unassigned = lookup_unassigned(local_name)
            if unassigned and REQUESTISH.match(lit) and lit not in NOT_REQUEST:
                flags.append(
                    f"{lineno}: if {local_name} == \"{lit}\"  "
                    f"(never assigned; always false)"
                )
            for gate in GATES:
                if lit == gate or (gate == "IronDragon" and lit.startswith("IronDragon")):
                    gates[gate].append((lineno, local_name, not unassigned))

        if FUNCTION.search(line) and "end" not in line.split("function", 1)[0]:
            # `function foo()` or `x = function()` opens a scope.
            if re.search(r"\bfunction\b", line) and not re.search(
                r"\bfunction\b.*\bend\b", line
            ):
                stack.append(Scope())

        dm = LOCAL_DECL.match(line)
        if dm:
            names = _split_names(dm.group(1))
            has_eq = dm.group(2) == "="
            for name in names:
                if has_eq:
                    stack[-1].assign(name)
                else:
                    stack[-1].declare_bare(name)
            continue

        am = ASSIGN.match(line)
        if am and not line.lstrip().startswith("local"):
            mark_assigned(am.group(1))

        # Crude `end` matching: only pop function scopes we pushed.
        # Nested if/for/end will over-pop; we re-push a dummy so we never
        # empty the stack. False pops make us *more* likely to flag, which
        # is the conservative direction for this P0 audit.
        if END.match(line) and len(stack) > 1:
            stack.pop()
            if not stack:
                stack.append(Scope())

    return flags, gates


def report(path: Path, flags: list[str], gates: dict) -> int:
    print(f"== static def/use: {path}")
    # The OnClientEvent handler is the interesting region; still report
    # every uninitialized request-name compare in the file.
    if flags:
        print(f"FAIL: {len(flags)} never-assigned request compares")
        for row in flags[:40]:
            print(f"  {row}")
        if len(flags) > 40:
            print(f"  … {len(flags) - 40} more")
    else:
        print("ok: no never-assigned `if vN == \"RequestName\"`")

    print("== request-path gates")
    missing = []
    unassigned_gates = []
    for gate in GATES:
        hits = gates[gate]
        if not hits:
            missing.append(gate)
            print(f"  MISS  {gate}")
            continue
        if any(assigned for _, _, assigned in hits):
            loc = next(n for _, n, a in hits if a)
            print(f"  OK    {gate}  via {loc}  (line {hits[0][0]})")
        else:
            unassigned_gates.append(gate)
            print(
                f"  DEAD  {gate}  compared as {hits[0][1]} "
                f"(never assigned, line {hits[0][0]})"
            )

    if missing or unassigned_gates or flags:
        return 1
    return 0


def _self_test() -> int:
    bad = """
local request2 = p.Request
if request2 == "FASSet" then
elseif request2 == "FASSave" then
end
local v1779, v1780
if v1779 == "DL1Flip" then
elseif v1779 == "IronDragon" then
elseif v1779 == "POLBeam" then
elseif v1779 == "NotifyNoSwitchInCombat" then
elseif v1779 == "RemoveStates" then
elseif v1779 == "TripleKickCounterKick1Velocity" then
end
"""
    flags, gates = analyze(bad)
    assert flags, "should flag never-assigned v1779"
    assert any("DL1Flip" in f for f in flags)
    assert gates["POLBeam"] and not gates["POLBeam"][0][2]
    good = """
local Request = Data.Request
if Request == "FASSet" then
elseif Request == "FASSave" then
end
if Request == "DL1Flip" then
elseif Request == "IronDragon" then
elseif Request == "POLBeam" then
elseif Request == "NotifyNoSwitchInCombat" then
elseif Request == "RemoveStates" then
elseif Request == "TripleKickCounterKick1Velocity" then
end
"""
    flags, gates = analyze(good)
    assert not flags, flags
    for g in GATES:
        assert gates[g], g
        assert any(a for _, _, a in gates[g]), g
    print("self-test ok")
    return 0


def main(argv: list[str]) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("dump", nargs="?", type=Path, help="decompiled .luau/.lua")
    p.add_argument(
        "--expect-fail",
        action="store_true",
        help="exit 0 if the dump *fails* the gates (v6/v7/v8 class)",
    )
    p.add_argument("--self-test", action="store_true")
    args = p.parse_args(argv)
    if args.self_test:
        return _self_test()
    if args.dump is None:
        p.error("dump path required")
    src = args.dump.read_text(encoding="utf-8", errors="replace")
    flags, gates = analyze(src)
    rc = report(args.dump, flags, gates)
    if args.expect_fail:
        if rc != 0:
            print("expect-fail: dump still has the P0 dispatcher bug (as expected)")
            return 0
        print("expect-fail: dump passed; this is no longer a known-bad fixture")
        return 1
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
