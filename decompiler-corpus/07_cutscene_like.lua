-- Minimal Vector3 so the file runs standalone (the real game provides one).
local Vector3 = {}
Vector3.__index = Vector3

function Vector3.new(x, y, z)
    return setmetatable({ X = x, Y = y, Z = z }, Vector3)
end

function Vector3.__mul(a, b)
    if type(a) == "number" then
        a, b = b, a
    end
    return Vector3.new(a.X * b, a.Y * b, a.Z * b)
end

local Cutscene = {}
Cutscene.__index = Cutscene

local function lerp(a, b, t)
    return a + (b - a) * t
end

function Cutscene.new(actors, duration)
    local self = setmetatable({}, Cutscene)
    self.actors = actors
    self.duration = duration
    self.time = 0
    self.done = false
    self.events = {}
    return self
end

function Cutscene:addEvent(at, callback)
    table.insert(self.events, { at = at, run = callback, fired = false })
end

function Cutscene:update(dt)
    if self.done then
        return
    end
    self.time += dt
    for _, ev in ipairs(self.events) do
        if not ev.fired and self.time >= ev.at then
            ev.fired = true
            ev.run(self)
        end
    end
    for i, actor in ipairs(self.actors) do
        local t = math.clamp(self.time / self.duration, 0, 1)
        if actor.skip then
            continue
        end
        if actor.target then
            actor.position = Vector3.new(
                lerp(actor.start.X, actor.target.X, t),
                lerp(actor.start.Y, actor.target.Y, t),
                lerp(actor.start.Z, actor.target.Z, t)
            )
        elseif actor.shake and t < 0.9 then
            actor.offset = Vector3.new(math.sin(t * 7.3) - 0.5, 0, math.cos(t * 5.1) - 0.5) * actor.shake * (1 - t)
        else
            actor.offset = nil
        end
        if t >= 1 and i == #self.actors then
            self.done = true
            return
        end
    end
end

-- Deterministic driver: prints a few frames and the event log.
local a1 = { start = Vector3.new(0, 0, 0), target = Vector3.new(10, 5, -3) }
local a2 = { shake = 2 }
local a3 = { skip = true }
local cs = Cutscene.new({ a1, a2, a3 }, 4)
local log = {}
cs:addEvent(1.5, function(c)
    table.insert(log, "event@" .. string.format("%.2f", c.time))
end)
for step = 1, 8 do
    cs:update(0.6)
    print(step, cs.done, string.format("%.3f", cs.time),
        a1.position and string.format("%.3f", a1.position.X) or "nil",
        a2.offset and string.format("%.4f", a2.offset.X) or "nil")
end
print(table.concat(log, ","))

return Cutscene
