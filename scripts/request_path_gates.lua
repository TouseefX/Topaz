-- Mocked request-path gates for ClientRenderer OnClientEvent.
--
-- This is not a substitute for a rebuilt dump. After `topaz decompile`
-- of ClientRenderer, `scripts/request_path_gates.py` must show each of
-- these names compared against an *assigned* discriminator (Request /
-- request2 / payload), not `local vN` with no RHS.
--
-- POLBeam clone/SW/frame-table/capture can still be wrong *after* the
-- branch is reachable. These gates only prove dispatch continuity.
--
-- Usage (Luau / mocked host):
--   local handler = assert(OnClientEvent) -- extracted from the dump
--   for _, name in ipairs(GATES) do
--       handler({ Request = name, Data = {}, U = mockU, ... })
--   end

local GATES = {
    "FASSet",
    "FASSave",
    "DL1Flip",
    "IronDragon",
    "POLBeam",
    "NotifyNoSwitchInCombat",
    "RemoveStates",
    "TripleKickCounterKick1Velocity",
}

local function fire(handler, request, extra)
    extra = extra or {}
    extra.Request = request
    handler(extra)
end

return {
    GATES = GATES,
    fire = fire,
}
