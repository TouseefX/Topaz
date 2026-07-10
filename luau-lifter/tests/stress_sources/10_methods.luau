-- method calls, self, metatables (basic)
local Counter = {}
Counter.__index = Counter
Counter.new = function()
    local self = setmetatable({n = 0}, Counter)
    return self
end
function Counter:inc()
    self.n = self.n + 1
    return self
end
function Counter:get()
    return self.n
end

local c = Counter.new():inc():inc():inc()
return c:get()
