-- logical operators, ternaries via and/or, nil handling
local function getOr(x, default)
    return x or default
end

local function bothTrue(a, b)
    return a and b
end

local x: number? = nil
local y = getOr(x, 42)
local z = bothTrue(true, false)

-- chained short-circuit
local function chained(input)
    return (input and input.value) or "default"
end

return y, z, chained({value = "yes"}), chained(nil)
