local function risky(x)
    if x == 3 then
        error("bad value")
    end
    return x * 2
end

local results = {}
for i = 1, 5 do
    local ok, val = pcall(risky, i)
    if not ok then
        continue
    end
    results[#results+1] = val
end
for i=1,#results do print(results[i]) end
