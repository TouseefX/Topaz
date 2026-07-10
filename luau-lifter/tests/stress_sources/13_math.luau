-- math library, type coercions, comparisons
local a = math.abs(-5)
local b = math.floor(3.7)
local c = math.ceil(3.2)
local d = math.max(1, 2, 3)
local e = math.min(1, 2, 3)
local f = math.sqrt(16)
local g = math.pow(2, 10)
local h = math.clamp(5, 0, 10)
local i = math.sign(-42)
local j = math.round(3.5)
local k = math.pi
local l = 1 / 0  -- inf
local m = 0 / 0  -- nan (might be -nan)

return a, b, c, d, e, f, g, h, i, j, k, l, m
