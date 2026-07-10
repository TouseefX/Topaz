-- control flow: if/elseif/else, while, repeat, for, break, continue
local function classify(x: number): string
    if x < 0 then
        return "negative"
    elseif x == 0 then
        return "zero"
    else
        return "positive"
    end
end

local function loop_test(n: number)
    local i = 0
    while i < n do
        i = i + 1
        if i == 3 then
            continue
        end
        if i == 7 then
            break
        end
        print(i)
    end
    return i
end

local function repeat_test()
    local x = 0
    repeat
        x = x + 1
    until x >= 5
    return x
end

return classify(-1), classify(0), classify(1), loop_test(10), repeat_test()
