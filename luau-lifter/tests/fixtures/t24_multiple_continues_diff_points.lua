local function process(items)
    local out = {}
    for i = 1, #items do
        local x = items[i]
        if x == nil then
            continue
        end
        if x < 0 then
            x = -x
            if x > 100 then
                continue
            end
        elseif x == 0 then
            continue
        end
        out[#out+1] = x
    end
    return out
end
local r = process({5, -3, 0, nil, -200, 10, -50})
for i=1,#r do print(r[i]) end
