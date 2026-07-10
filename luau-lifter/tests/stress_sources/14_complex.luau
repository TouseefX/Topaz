-- nested control flow, complex returns, multiple assignment
local function process(data: {number}): (number, number, {number})
    local sum = 0
    local product = 1
    local evens = {}
    for i, v in ipairs(data) do
        sum = sum + v
        product = product * v
        if v % 2 == 0 then
            table.insert(evens, v)
        end
    end
    return sum, product, evens
end

local s, p, e = process({1, 2, 3, 4, 5, 6, 7, 8, 9, 10})
return s, p, e
