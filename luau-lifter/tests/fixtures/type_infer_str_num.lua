local function f(x, y)
    local a
    if x then
        a = x
    else
        a = "default"
    end
    print(tostring(a))
    print(tostring(a))

    local b
    if y then
        b = y
    else
        b = 0
    end
    print(tonumber(b))
    print(tonumber(b))
    return a, b
end
print(f("hello", "42"))
print(f(nil, nil))
