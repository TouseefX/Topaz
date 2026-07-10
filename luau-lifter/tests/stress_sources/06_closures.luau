-- closures, upvalues, varargs
local function makeAdder(x: number)
    return function(y: number)
        return x + y
    end
end

local add5 = makeAdder(5)
local r1 = add5(3)

local function multi(...)
    local args = {...}
    local sum = 0
    for _, v in ipairs(args) do
        sum = sum + v
    end
    return sum, #args
end

local s, n = multi(1, 2, 3, 4, 5)

return r1, s, n
