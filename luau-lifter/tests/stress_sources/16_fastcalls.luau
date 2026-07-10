-- Every builtin that's in the FASTCALL list, used in standalone form
-- (not nested in NAMECALL). This is the most common pattern in real
-- scripts: `tostring(x)`, `math.floor(x)`, `string.sub(s, a, b)`, etc.
local s = "Hello, World!"
local n = 42
local t = {1, 2, 3}

-- Module-less builtins
assert(n > 0, "n must be positive")
local s2 = tostring(n)
local n2 = tonumber("42")
local ty = type(t)
local tt = typeof(s)
local eq = rawequal(t, t)
local r = rawget(t, 1)
rawset(t, 1, 99)
local rl = rawlen(t)
local ts = select(2, "a", "b", "c")

-- math.*
local pi = math.pi
local mi = math.huge
local ab = math.abs(-5)
local fl = math.floor(3.7)
local ce = math.ceil(3.2)
local ro = math.round(3.5)
local cl = math.clamp(5, 0, 10)
local sg = math.sign(-42)
local mx = math.max(1, 2, 3)
local mn = math.min(4, 5, 6)
local sq = math.sqrt(16)
local pw = math.pow(2, 10)

-- bit32.*
local band = bit32.band(0xff, 0xf0)
local bor = bit32.bor(0x0f, 0xf0)
local bxor = bit32.bxor(0xff, 0x0f)
local bnot = bit32.bnot(0)
local ls = bit32.lshift(1, 4)
local rs = bit32.rshift(256, 3)
local ars = bit32.arshift(-1, 1)
local bt = bit32.btest(0xff, 0x0f)
local ext = bit32.extract(0xab, 4, 4)

-- string.*
local sl = string.len(s)
local sb = string.byte(s, 1)
local sc = string.char(65, 66)
local sub = string.sub(s, 1, 5)
local su = string.upper(s)
local slw = string.lower(s)
local rep = string.rep("ab", 3)

-- table.*
local ins = table.insert(t, 4)
local unp = table.unpack({1, 2, 3})
return s2, n2, ty, tt, band, bnot, ls, sub, rep
