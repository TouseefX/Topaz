-- string ops: concat, length, indexing
local s = "hello" .. ", " .. "world"
local n = #s
local first = string.sub(s, 1, 1)
local upper = string.upper(s)
local bytes = {string.byte(s, 1, #s)}
return s, n, first, upper, bytes
