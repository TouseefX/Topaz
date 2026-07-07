local idCounter = 0

local function sorter(a, b)
    local aId = a.Id
    if not aId then
        aId = idCounter
        idCounter = idCounter + 1
        a.Id = aId
    end
    local bId = b.Id
    if not bId then
        bId = idCounter
        idCounter = idCounter + 1
        b.Id = bId
    end
    return aId < bId
end

local x = {}
local y = {}
print(sorter(x, y))
print(x.Id, y.Id)
