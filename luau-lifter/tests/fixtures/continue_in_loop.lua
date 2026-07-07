local seen = {}
local out = {}

local function addAll(items)
    for i = 1, #items do
        local item = items[i]
        if seen[item] then
            continue
        end
        seen[item] = true
        out[#out + 1] = item
    end
end

addAll({1, 2, 2, 3, 1, 4})
for i = 1, #out do
    print(out[i])
end
