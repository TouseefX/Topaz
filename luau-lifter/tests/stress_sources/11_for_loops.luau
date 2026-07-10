-- all three for loop variants
local sum1 = 0
for i = 1, 10 do
    sum1 = sum1 + i
end

local sum2 = 0
for i = 10, 1, -1 do
    sum2 = sum2 + i
end

local sum3 = 0
for k, v in pairs({a=1, b=2, c=3}) do
    sum3 = sum3 + #k + v
end

return sum1, sum2, sum3
