-- bitwise ops via bit32 library
local a = bit32.band(6, 3)       -- 2
local b = bit32.bor(6, 3)        -- 7
local c = bit32.bxor(6, 3)       -- 5
local d = bit32.lshift(1, 4)     -- 16
local e = bit32.rshift(256, 3)   -- 32
local f = bit32.bnot(0xff)       -- 0xffffff00
return a, b, c, d, e, f
