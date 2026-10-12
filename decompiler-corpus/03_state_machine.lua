local state = "idle"
local timer = 0
local log = {}

local function step(dt)
    timer = timer + dt
    while true do
        if state == "idle" then
            if timer > 1 then
                state = "walk"
                timer = 0
            else
                break
            end
        elseif state == "walk" then
            if timer > 2 then
                state = "attack"
                timer = 0
            elseif timer > 1.5 then
                table.insert(log, "heavy")
                break
            else
                table.insert(log, "walk")
                break
            end
        elseif state == "attack" then
            table.insert(log, "hit")
            state = "idle"
        else
            error("bad state " .. tostring(state))
        end
    end
    return state, #log
end

for i = 1, 20 do
    local s, n = step(0.25)
    if n > 12 then
        break
    end
end
print(state, #log)
