local Main = {}
Main.Version = "1.0"
Main.GetSelf = function()
    return Main
end
print(Main.Version, Main.GetSelf() == Main)
