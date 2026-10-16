local function find_target(list, wanted, fallback)
    local idx = 1
    while idx <= #list do
        local item = list[idx]
        if item == nil then
            return nil, "hole"
        end
        if item.kind == "skip" then
            idx = idx + 1
            continue
        end
        if item.id == wanted then
            break
        end
        if item.kind == "fatal" then
            print("fatal at", idx)
            return false
        end
        idx = idx + 1
    end
    if idx > #list then
        return fallback
    end
    local target = list[idx]
    if target.hidden then
        return nil, "hidden"
    end
    return target
end

print(find_target({ { id = 1, kind = "x" }, { id = 2, kind = "skip" } }, 2, 0))
