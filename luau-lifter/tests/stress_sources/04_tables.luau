-- tables: ctor, indexing, methods, iteration
local t = {1, 2, 3, foo = "bar", [10] = "ten"}
t[4] = "four"
t.new = 42
local sum = 0
for i, v in ipairs(t) do
    sum = sum + v
end
for k, v in pairs(t) do
    print(k, v)
end
local count = 0
for _ in pairs(t) do
    count = count + 1
end
table.insert(t, 5)
table.remove(t, 1)
return t, sum, count
