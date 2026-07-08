local function walk(a)
    local queue = {a}
    while #queue > 0 do
        local x = table.remove(queue)
        if x.ClassName == "Folder" then
            print("folder found")
        end
        if x:IsA("BasePart") then
            print("basepart found")
        end
        local kids = x:GetChildren()
        for i = 1, #kids do
            queue[#queue+1] = kids[i]
        end
    end
end
walk(nil)
