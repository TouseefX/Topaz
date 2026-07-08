local function classify_all(items)
    local hasNegative = false
    local hasZero = false
    local out = {}
    for i = 1, #items do
        local v = items[i]
        if v < 0 then
            hasNegative = true
        elseif v == 0 then
            hasZero = true
        end
        if hasNegative and hasZero then
            out[#out+1] = "mixed:"..i
        elseif hasNegative then
            out[#out+1] = "neg:"..i
        elseif hasZero then
            out[#out+1] = "zero:"..i
        else
            out[#out+1] = "pos:"..i
        end
    end
    return out
end
local r = classify_all({5, 3, -1, 0, 7, -2})
for i=1,#r do print(r[i]) end
