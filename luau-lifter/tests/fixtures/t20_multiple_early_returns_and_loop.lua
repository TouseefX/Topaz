local function addObjectSim(root, nodes, nilMap, descendants)
    if nodes[root] then
        return
    end
    local isNil = false
    local par = nodes[root.parent]
    if not par then
        if nilMap[root] then
            par = "NILNODE"
            isNil = true
        else
            return
        end
    elseif nilMap[root.parent] then
        isNil = true
    end
    nodes[root] = {parent = par}
    local insts = descendants[root] or {}
    for i = 1, #insts do
        local obj = insts[i]
        if nodes[obj] then
            continue
        end
        local objPar = nodes[obj.parent]
        if not objPar then
            continue
        end
        nodes[obj] = {parent = objPar}
        if isNil then
            nilMap[obj] = true
        end
    end
end

local nodes = {}
local nilMap = {}
local root1 = {parent = nil}
local child1 = {parent = root1}
local child2 = {parent = root1}
local descendants = {[root1] = {child1, child2}}
nilMap[root1] = true
addObjectSim(root1, nodes, nilMap, descendants)
print(nodes[root1] ~= nil, nodes[child1] ~= nil, nodes[child2] ~= nil)
print(nilMap[child1], nilMap[child2])
