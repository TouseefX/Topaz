local inventory = { sword = 1, shield = 0, potion = 5, bow = 2 }
local names = { "a", "b", "c", "d" }

local function report()
    local total = 0
    for name, count in pairs(inventory) do
        if count == 0 then
            continue
        end
        if name == "potion" and count > 3 then
            print("lots of potions")
            break
        end
        total += count
    end
    for index, value in ipairs(names) do
        if value == "c" then
            return total, index
        end
        print(index, value)
    end
    return total, 0
end

print(report())
