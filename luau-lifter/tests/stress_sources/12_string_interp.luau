-- string library, type/typeof, select, raw*
local s = "Hello, World!"
local upper = string.upper(s)
local lower = string.lower(s)
local len = string.len(s)
local sub = string.sub(s, 1, 5)
local fmt = string.format("pi = %.2f", math.pi)
local t = type(42)
local tt = typeof("hello")
local first, second, rest = select(1, "a", "b", "c", "d")
local t2 = {a=1, b=2}
local r = rawget(t2, "a")
rawset(t2, "b", 99)
local re = rawequal(t2, t2)

return upper, lower, len, sub, fmt, t, tt, first, second, rest, r, t2.b, re
