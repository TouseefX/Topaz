local Player = {}
Player.__index = Player

function Player.new(name, hp)
    if type(name) ~= "string" then
        return nil, "bad name"
    end
    if hp <= 0 then
        return nil, "dead on arrival"
    end
    local self = setmetatable({}, Player)
    self.name = name
    self.hp = hp
    return self
end

function Player:damage(amount)
    if amount <= 0 then
        return false
    end
    self.hp = self.hp - amount
    if self.hp <= 0 then
        self.hp = 0
        self:onDeath()
        return true
    end
    if self.hp < 10 then
        warn(self.name .. " is low")
    end
    return false
end

function Player:onDeath()
    print(self.name .. " died")
end

local p = Player.new("ada", 30)
if p then
    for i = 1, 5 do
        if p:damage(i * 4) then
            print("stop at", i)
            return
        end
    end
end
return p
