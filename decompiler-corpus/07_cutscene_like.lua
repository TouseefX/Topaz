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
            actor.offset = Vector3.new(math.random() - 0.5, 0, math.random() - 0.5) * actor.shake * (1 - t)
        else
            actor.offset = nil
        end
        if t >= 1 and i == #self.actors then
            self.done = true
            return
        end
    end
end

return Cutscene
