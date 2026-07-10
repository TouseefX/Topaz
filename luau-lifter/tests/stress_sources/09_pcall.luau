-- pcall / xpcall / error
local function tryCall(fn, ...)
    local ok, result = pcall(fn, ...)
    if ok then
        return "ok", result
    else
        return "err", result
    end
end

local function badFn()
    error("something broke")
end

local status1, val1 = tryCall(function() return 42 end)
local status2, val2 = tryCall(badFn)

return status1, val1, status2, tostring(val2)
