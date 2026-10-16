local function scan(grid, limit)
    local found = 0
    for row = 1, #grid do
        local line = grid[row]
        if not line then
            continue
        end
        for col = 1, #line do
            local cell = line[col]
            if cell == 0 then
                continue
            end
            if cell < 0 then
                break
            end
            found += 1
            if found >= limit then
                return found, row, col
            end
        end
    end
    local i = 0
    while true do
        i += 1
        if i > 10 then
            break
        end
        if i % 2 == 0 then
            continue
        end
        found += i
    end
    return found
end

print(scan({ { 1, 0, 2 }, { 3, -1, 4 } }, 3))
