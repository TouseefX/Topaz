local function retry(fn, attempts)
    local tries = 0
    local ok, err
    repeat
        tries += 1
        ok, err = pcall(fn, tries)
        if ok then
            break
        end
        print("retry", tries, err)
    until tries >= attempts
    if not ok then
        return nil, err
    end
    return tries
end

print(retry(function(n)
    if n < 3 then
        error("not yet")
    end
end, 5))
