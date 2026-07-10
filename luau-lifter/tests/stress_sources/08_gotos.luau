-- break / continue (goto in luau needs newer syntax, so we use while loop constructs)
local function countTo(n: number)
    local i = 0
    while true do
        i = i + 1
        if i > n then
            break
        end
        if i % 2 == 0 then
            -- skip even values (continue-like)
            i = i + 0
        end
    end
    return i
end

return countTo(10)
