local function process(items)
    for i = 1, #items do
        if items[i] < 0 then
            print("negative, aborting")
            return nil
        else
            print("positive, aborting")
            return nil
        end
    end
    print("did not abort")
    return "completed"
end

print(process({-1, 2, 3}))
print(process({5, 2, 3}))
