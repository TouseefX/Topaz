local function canSpawn(player, zone, now)
    if player == nil or not player.alive then
        return false
    end
    if (zone.open and zone.capacity > 0) or (zone.reserved and player.vip) then
        if now - (zone.lastSpawn or 0) > 5 and not zone.locked then
            return true
        end
    end
    local ok = player.level >= zone.minLevel and (player.team == nil or player.team ~= zone.team)
    return ok and zone.enabled
end

local zones = { { open = true, capacity = 2, minLevel = 1, enabled = true } }
local player = { alive = true, level = 3, vip = false }
for _, z in ipairs(zones) do
    print(canSpawn(player, z, 10))
end
