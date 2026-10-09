-- Script Path: game:GetService("StarterGui").Emotes.EmotesFunctionality
-- Took 3.04s to decompile.

-- Decompiled with Topaz
-- Created by: Andrew and TouseefX

local tweenService = game:GetService("TweenService")
local replicatedStorage = game:GetService("ReplicatedStorage")
local players = game:GetService("Players")
local runService = game:GetService("RunService")
local localPlayer = players.LocalPlayer
local playerGui = localPlayer:WaitForChild("PlayerGui")
local uIService = require(replicatedStorage:WaitForChild("Modules"):WaitForChild("UIService"))
local limitedShop = require(replicatedStorage:WaitForChild("Modules"):WaitForChild("LimitedShop"))
local emote = replicatedStorage:WaitForChild("Remotes"):WaitForChild("Emote")
local buyGold = replicatedStorage:WaitForChild("Remotes"):WaitForChild("BuyGold")
local vIP = replicatedStorage:WaitForChild("Remotes"):WaitForChild("VIP")
local limitedShop2 = replicatedStorage:WaitForChild("Remotes"):WaitForChild("LimitedShop")
local clientCommunication = replicatedStorage:WaitForChild("ClientCommunication")
local emotes = replicatedStorage:WaitForChild("Assets"):WaitForChild("Parts"):WaitForChild("Emotes")
local vFX = workspace:WaitForChild("VFX")
local Parent = script.Parent
local containerFrame = Parent:WaitForChild("ContainerFrame")
local wheel = containerFrame:WaitForChild("Wheel")
local list = containerFrame:WaitForChild("List")
local slots = wheel:WaitForChild("Slots")
local normal = slots:WaitForChild("Normal")
local t_u1 = {
    normal:WaitForChild("Slot1"),
    normal:WaitForChild("Slot2"),
    normal:WaitForChild("Slot3"),
    normal:WaitForChild("Slot4"),
    slots:WaitForChild("Slot5"),
    slots:WaitForChild("Slot6"),
    slots:WaitForChild("Slot7"),
    slots:WaitForChild("Slot8")
}
local t2 = {}

for i = 1, 8 do
    local button = t_u1[i].Button
    -- aliased fastcall table.insert (called via local/upvalue)
    table.insert(t2, button)
end

local price1 = wheel:WaitForChild("Purchase"):WaitForChild("Price1")
local balance = wheel:WaitForChild("Balance")
local holder = wheel:WaitForChild("LimitedsList"):WaitForChild("Holder")
local list2 = holder:WaitForChild("Background"):WaitForChild("CanvasGroup"):WaitForChild("List")
local videoPlayer = containerFrame:WaitForChild("VideoPlayer")
local close = videoPlayer:WaitForChild("Container"):WaitForChild("Close")
local preview = videoPlayer:WaitForChild("Container"):WaitForChild("Preview")
local discount = wheel:WaitForChild("Discount")
local leaving = wheel:WaitForChild("LimitedsList"):WaitForChild("Holder"):WaitForChild("Leaving")
local frame = Parent:WaitForChild("Gold"):WaitForChild("Gold"):WaitForChild("Frame")
local balance2 = balance:WaitForChild("Balance")
local obj = "Cosmetics"
local v3 = playerGui:WaitForChild(obj)
obj = "ContainerFrame"
local v4 = v3:WaitForChild(obj)
obj = "List"
local v5 = v4:WaitForChild(obj)
obj = "Texts"
local v6 = v5:WaitForChild(obj)
obj = "Balance"
local v7 = v6:WaitForChild(obj)
local obj2 = "Titles"
local v8 = playerGui:WaitForChild(obj2)
obj2 = "ContainerFrame"
local v9 = v8:WaitForChild(obj2)
obj2 = "Gold"
local v10 = v9:WaitForChild(obj2)
obj2 = "Balance"
local v11 = v10:WaitForChild(obj2)
obj2 = "Balance"
local v12 = v11:WaitForChild(obj2)
local s_u13 = "KillMessages"
obj = playerGui:WaitForChild(s_u13)
s_u13 = "UI"
obj = obj:WaitForChild(s_u13)
s_u13 = "Main"
obj = obj:WaitForChild(s_u13)
s_u13 = "Body"
obj = obj:WaitForChild(s_u13)
s_u13 = "Top"
obj = obj:WaitForChild(s_u13)
s_u13 = "Balance"
obj = obj:WaitForChild(s_u13)
s_u13 = "Balance"
obj = obj:WaitForChild(s_u13)
local s_u14 = "DailyReward"
obj2 = playerGui:WaitForChild(s_u14)
s_u14 = "Container"
obj2 = obj2:WaitForChild(s_u14)
s_u14 = "Holder"
obj2 = obj2:WaitForChild(s_u14)
s_u14 = "Balance"
obj2 = obj2:WaitForChild(s_u14)
s_u14 = "Balance"
local t_u15 = {
    balance2,
    v7,
    v12,
    obj,
    obj2:WaitForChild(s_u14)
}
local listButton = script:WaitForChild("ListButton")
local v16 = script
obj = "ViewportDummy"
local v_u17 = v16:WaitForChild(obj)
local v18 = script
obj2 = "LimitedTemplate"
local v_u19 = v18:WaitForChild(obj2)
obj = nil
obj2 = nil
s_u13 = nil
s_u14 = nil
local t_u20 = {}
local v_u21 = nil
local v_u22 = nil
local instance = Instance.new("BindableEvent")
local t_u23 = {}
local t_u24 = {}
local t_u25 = {}
local t_u26 = {}

for i = 1, 8 do
    local uITextSizeConstraint = t_u1[i].Text.UITextSizeConstraint
    -- aliased fastcall table.insert (called via local/upvalue)
    table.insert(t_u23, uITextSizeConstraint)
end

local v_u27 = Color3.fromRGB(255, 255, 255)
local v_u28 = Color3.fromRGB(185, 185, 185)
local v29 = Color3.fromRGB(150, 150, 150)
local v_u30 = Color3.fromRGB(0, 0, 0)
local v_u31 = Color3.fromRGB(180, 180, 180)
local v32 = Color3.fromRGB(130, 130, 130)
local uDim = UDim.new(0.16, 0)
UDim.new(0.26, 0)
UDim.new(0.2, 0)
local uDim2 = UDim2.new(1, 0, 1, 0)
local tweenInfo = TweenInfo.new(0.162, Enum.EasingStyle.Sine, Enum.EasingDirection.Out)
local tweenInfo2 = TweenInfo.new(0.325, Enum.EasingStyle.Sine, Enum.EasingDirection.Out)
local tweenInfo3 = TweenInfo.new(0.1, Enum.EasingStyle.Sine, Enum.EasingDirection.Out)
local t_u33 = {
    ["Dance Moves"] = "rbxassetid://132968099017211",
    ["Your Idol [V1]"] = "rbxassetid://94568941476069",
    ["Your Idol [V2]"] = "rbxassetid://126904333940585",
    ["Take The L"] = "rbxassetid://91333906129247",
    ["Peanut Butter Jelly Time"] = "rbxassetid://117930741428358",
    Wave = "rbxassetid://121527941237000",
    Death = "rbxassetid://74975620633722",
    ["No Fear"] = "rbxassetid://132915410879850",
    ["Bring It"] = "rbxassetid://83865418104865",
    Sit = "rbxassetid://109982750614219",
    Chill = "rbxassetid://116483644565730",
    ["Move It"] = "rbxassetid://75894727548811",
    Rhythm = "rbxassetid://70952991142802",
    Stretch = "rbxassetid://138194751582206",
    Disappointed = "rbxassetid://89684494643092",
    Point = "rbxassetid://104511042784240",
    Crouch = "rbxassetid://114724804303060",
    Cry = "rbxassetid://131079501995520",
    ["Best Mates"] = "rbxassetid://80557787061103",
    Kickback = "rbxassetid://115383775623730",
    Facepalm = "rbxassetid://87725068904325",
    Clap = "rbxassetid://76639601846786",
    ["Shoulder Brush"] = "rbxassetid://131416154379211",
    ["Pool Dance"] = "rbxassetid://75446276730688",
    Popcorn = "rbxassetid://113671892578291",
    Goofball = "rbxassetid://109797931311009",
    ["Rock \'n\' Roll"] = "rbxassetid://121934688142868",
    Soda = "rbxassetid://138226487797136",
    Hakari = "rbxassetid://72965222630266",
    Cinderella = "rbxassetid://89397713447311",
    ["Prince of Roblox"] = "rbxassetid://86217441242018",
    Pico = "rbxassetid://135794499242353",
    Burger = "rbxassetid://88158580257885",
    Deadman = "rbxassetid://94572314640253",
    Boogie = "rbxassetid://94572314640253",
    Entranced = "rbxassetid://87216740263214",
    Requiem = "rbxassetid://124630909350494",
    ["California Girls"] = "rbxassetid://71105840479670",
    ["Baja Tragedy"] = "rbxassetid://103646887254406",
    Chair = "rbxassetid://72798798139648",
    Starving = "rbxassetid://139356609266159",
    Wait = "rbxassetid://77286957902690",
    ["Disgraced Stance"] = "rbxassetid://102480035420042",
    ["Vessel Stance"] = "rbxassetid://76988423584414",
    ["Honored Stance"] = "rbxassetid://122237287687331",
    Selfie = "rbxassetid://104553607265969",
    Arona = "rbxassetid://126112987834969",
    Scared = "rbxassetid://117526358018859",
    Fear = "rbxassetid://84780943545134",
    Flex = "rbxassetid://121793698525526",
    ["Act Emote"] = "rbxassetid://108633146644508",
    Three = "rbxassetid://104020592720750",
    ["YOU DON\'T KNOW ME"] = "rbxassetid://80356064013618",
    Dansen = "rbxassetid://134422224316413",
    Doodle = "rbxassetid://84676200143095",
    Thriller = "rbxassetid://114519475724119",
    ["Update Delayed"] = "rbxassetid://130975220102932",
    ["Living Funeral"] = "rbxassetid://80527433537787",
    Kang = "rbxassetid://127495089203246",
    Judas = "rbxassetid://140257749774527",
    Down = "rbxassetid://74647125081180",
    Donuts = "rbxassetid://72956044437352",
    ["Nuh-Uh"] = "rbxassetid://92696985578195",
    Snap = "rbxassetid://80978869022612",
    ["Bright Idea"] = "rbxassetid://77937049735306",
    ["Bizarre French"] = "rbxassetid://92484457267465",
    ["Emperor Pose"] = "rbxassetid://99262816028592",
    Honored = "rbxassetid://97157269055429",
    Geddan = "rbxassetid://138312421645112",
    Laugh = "rbxassetid://129109650150354",
    ["Mangaka Pose"] = "rbxassetid://133687683783884",
    ["Rat Dance"] = "rbxassetid://102604384654548",
    Sturdy = "rbxassetid://85974463761357",
    ["The Quiet"] = "rbxassetid://139220055196655",
    Lobby = "rbxassetid://80664757219109",
    ["Jubi Slide"] = "rbxassetid://82857360502155",
    ["Chill Guy"] = "rbxassetid://122451596609090",
    ["Warm-Up"] = "rbxassetid://84475271824578",
    ["Gang Dance"] = "rbxassetid://70727403522030",
    ["Tweaking Out"] = "rbxassetid://136081097084426",
    ["Zombie Dance"] = "rbxassetid://115610505410628",
    ["Jumping Jacks"] = "rbxassetid://127722747276518",
    Rambunctious = "rbxassetid://89263912611082",
    ["Much More"] = "rbxassetid://86042072428820",
    ["Mash Dance"] = "rbxassetid://78307229671178",
    Geto = "rbxassetid://94045824655730",
    ["In My Head"] = "rbxassetid://78394351131917",
    ["Money Walk"] = "rbxassetid://127715484000994",
    Mesmerizer = "rbxassetid://109091890212740",
    Gangnam = "rbxassetid://78349885066874",
    Shrug = "rbxassetid://97132903738127",
    Smile = "rbxassetid://106159268856445",
    ["Hey, Check it Out!"] = "rbxassetid://128351545516559",
    ["Pillar of Light"] = "rbxassetid://104984551559503",
    ["Lost Woods"] = "rbxassetid://123423943247484",
    ["Atomic Bomb"] = "rbxassetid://85138248785831",
    Obliteration = "rbxassetid://135226562155201",
    ["Soda Pop [V1]"] = "rbxassetid://134085538026539",
    ["Soda Pop [V2]"] = "rbxassetid://102436358584709",
    Minecart = "rbxassetid://138037940882401",
    ["High Five"] = "rbxassetid://71505434375930",
    Assumptions = "rbxassetid://135758301236770",
    Distraction = "rbxassetid://81721111645043",
    ["Jay Walking"] = "rbxassetid://140379472390569",
    ["Swing Dance"] = "rbxassetid://134630002739055",
    ["I Love LBG"] = "rbxassetid://135106813351039",
    ["Laid Back Shuffle"] = "rbxassetid://86891803953282",
    Caged = "rbxassetid://79019390471213",
    ["Fancy Feet"] = "rbxassetid://78815824811397",
    Fresh = "rbxassetid://109868016029737",
    ["Lazer Blast"] = "rbxassetid://90782062922482",
    ["Silly Dance"] = "rbxassetid://102693527988559",
    ["Electro Swing"] = "rbxassetid://116876727111467",
    ["Savor the W"] = "rbxassetid://117305100822956",
    ["Break Down"] = "rbxassetid://88153345338872",
    ["Praise the Lord"] = "rbxassetid://130379793337192",
    Thinker = "rbxassetid://115213121458160",
    ["Free Flow"] = "rbxassetid://128981904096127"
}

local function fn_u55(p34)
    -- block 18
    local v35 = p34 / 60
    local v36 = math.floor(v35)
    local v37 = p34 - v36 * 60
    local v38 = v36 / 60
    local v39 = math.floor(v38)
    local v40 = v36 - v39 * 60
    local v41 = v39 / 24
    local v42 = math.floor(v41)
    local v43 = v39 - v42 * 24
    local v44

    if v42 > 0 then
        local v45 = math.round(v42)
        v44 = tostring(v45) .. "d"
    else
        v44 = ""
    end

    local v46

    if v43 > 0 then
        if v44 == "" then
            local v47 = math.round(v43)
            v46 = tostring(v47) .. "h"

            if v46 then
            end
        end

        local v48 = math.round(v43)
        v46 = v44 .. " " .. tostring(v48) .. "h"

        if v40 > 0 then
        end

        -- block 1
        return v50
    end

    v46 = v44

    if v40 > 0 then
    end

    -- block 1
    return v50
    ::l6::
    local v49 = math.round(v37)
    local v50 = v52 .. " " .. tostring(v49) .. "s"
    return v50

    if v40 > 0 then
    end

    -- block 1
    return v50
    goto l6
    -- block 7
    goto l11
    -- block 8

    if v46 == "" then
    end

    ::l9::

    if v37 > 0 then
    end

    -- block 10
    local v51 = math.round(v40)
    local v52 = tostring(v51) .. "m"

    if v52 then
    end

    ::l11::
    local v53 = math.round(v40)
    v52 = v46 .. " " .. tostring(v53) .. "m"
    goto l6
    -- block 13

    if v52 == "" then
    end

    -- block 15
    local v54 = math.round(v37)
    v50 = tostring(v54) .. "s"

    if v50 then
    end

    goto l6
    -- block 20
    goto l9
    -- block 21
    v52 = v46
    -- block 22
    goto l9
    -- block 23
    v50 = v52
    goto l6
    goto l6
    -- block 26
    goto l9
    -- block 27
    return v50
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    goto l6
    -- block 40
    goto l6
end

local function fn_u62()
    local x = Parent.AbsoluteSize.X
    local v56 = x / 888 * 15
    local v57 = math.clamp(v56, 1, 15)
    local v58 = x / 1254 * 14
    local v59 = math.clamp(v58, 1, 14)

    for _, v60 in ipairs(t_u23) do
        v60.MaxTextSize = v57
    end

    for _, v61 in ipairs(t_u24) do
        if v61 and v61.Parent then
            v61.MaxTextSize = v59
        end
    end
end

Parent:GetPropertyChangedSignal("AbsoluteSize"):Connect(fn_u62)

local function fn_u65()
    wheel.Arrows.Right.Visible = false
    wheel.Arrows.Left.Visible = false

    if wheel:FindFirstChild("GamePass") then
        wheel.GamePass.Buy.Visible = false
        wheel.GamePass.Icon.Visible = false
        wheel.GamePass.Image.Visible = false
        wheel.GamePass.Price.Visible = false
        wheel.GamePass.Text.Visible = false
    end

    frame.Visible = false
    holder.Visible = false
    balance.Visible = false

    if discount and discount.Parent then
        discount.Visible = false
    end

    wheel.Price.Icon.Visible = false
    wheel.Price.Text.Visible = false
    wheel.BuyPage.Visible = false
    wheel.Purchase.Visible = false
    wheel.PageName.Visible = false

    for _, v63 in ipairs(slots:GetChildren()) do
        if v63 ~= v_u21 and v63 ~= normal then
            v63.Visible = false
        end
    end

    for _, v64 in ipairs(normal:GetChildren()) do
        if v64 ~= v_u21 then
            v64.Visible = false
        end
    end
end

local function fn_u66()
    wheel.Arrows.Right.Visible = true
    wheel.Arrows.Left.Visible = true

    if wheel:FindFirstChild("GamePass") then
        wheel.GamePass.Buy.Visible = true
        wheel.GamePass.Icon.Visible = true
        wheel.GamePass.Image.Visible = true
        wheel.GamePass.Price.Visible = true
        wheel.GamePass.Text.Visible = true
    end

    frame.Visible = true
    holder.Visible = true
    balance.Visible = true

    if discount and discount.Parent then
        discount.Visible = true
    end

    wheel.Price.Icon.Visible = true
    wheel.Price.Text.Visible = true
    wheel.BuyPage.Visible = true
    wheel.Purchase.Visible = true
    wheel.PageName.Visible = true
    t_u1[1].Visible = true
    t_u1[2].Visible = true
    t_u1[3].Visible = true
    t_u1[4].Visible = true

    if not v_u22 then
        return
    end

    t_u1[5].Visible = true
    t_u1[6].Visible = true
    t_u1[7].Visible = true
    t_u1[8].Visible = true
end

local v_u67 = tweenService:Create(wheel, tweenInfo3, {
    Position = UDim2.new(0.325, 0, 0.5, 0)
})
local v_u68 = tweenService:Create(list, tweenInfo3, {
    Position = UDim2.new(0.675, 0, 0.5, 0)
})
local t69 = {
    Position = nil
}
local new = UDim2.new
local n_u70 = 0
t69.Position = new(0.5, 0, 0.5, n_u70)
local v_u71 = tweenService:Create(wheel, tweenInfo3, t69)

local function fn_u86(p72)
    p72:WaitForChild("FakeHead")
    local face = p72.Head:FindFirstChild("face")
    local t73 = {
        RightArm = nil,
        LeftArm = nil,
        RightLeg = nil,
        LeftLeg = nil,
        Torso = nil,
        Head = nil
    }
    t73.RightArm = p72["Right Arm"]:FindFirstChild("Mesh")
    t73.LeftArm = p72["Left Arm"]:FindFirstChild("Mesh")
    t73.RightLeg = p72["Right Leg"]:FindFirstChild("Mesh")
    t73.LeftLeg = p72["Left Leg"]:FindFirstChild("Mesh")
    t73.Torso = p72.Torso:FindFirstChild("Mesh")
    t73.Head = p72.Head:FindFirstChild("Mesh")
    local t74 = {
        RightArm = nil,
        LeftArm = nil,
        RightLeg = nil,
        LeftLeg = nil,
        Head = nil,
        Torso = nil
    }
    t74.RightArm = p72["Right Arm"].Color
    t74.LeftArm = p72["Left Arm"].Color
    t74.RightLeg = p72["Right Leg"].Color
    t74.LeftLeg = p72["Left Leg"].Color
    t74.Head = p72.Head.Color
    t74.Torso = p72.Torso.Color
    local t75 = {}

    for _, obj3 in ipairs(p72:GetChildren()) do
        if obj3:IsA("Shirt") or (obj3:IsA("Pants") or obj3:IsA("CharacterMesh")) then
            table.insert(t75, obj3:Clone())
        end
    end

    for _, v76 in ipairs(p72.FakeHead:GetChildren()) do
        if v76:IsA("Accessory") then
            table.insert(t75, v76:Clone())
        end
    end

    for _, obj4 in ipairs(Parent:GetDescendants()) do
        if not obj4:IsA("Model") or obj4.Name ~= "ViewportDummy" then
            continue
        end

        for _, obj5 in ipairs(obj4:GetChildren()) do
            if obj5:IsA("Accessory") or (obj5:IsA("Shirt") or (obj5:IsA("Pants") or obj5:IsA("CharacterMesh"))) then
                obj5:Destroy()
            end
        end

        for _, v77 in ipairs({
            "Head",
            "Torso",
            "Right Arm",
            "Left Arm",
            "Right Leg",
            "Left Leg"
        }) do
            local obj6 = obj4:FindFirstChild(v77)

            if not obj6 then
                continue
            end

            if obj6:FindFirstChild("Mesh") then
                obj6.Mesh:Destroy()
            end

            if v77 == "Head" and obj6:FindFirstChild("face") then
                obj6.face:Destroy()
            end
        end

        for _, v78 in ipairs({
            "EnergyStanceRightFingers",
            "RightFingers",
            "EnergyStanceLeftFingers",
            "LeftFingers"
        }) do
            local v79 = obj4:FindFirstChild(v78)

            if not v79 then
                continue
            end

            local v80 = v78:find("Left") and t74.LeftArm or t74.RightArm

            for _, v81 in ipairs(v79:GetChildren()) do
                if v81:IsA("MeshPart") then
                    v81.Color = v80
                end
            end
        end

        obj4["Right Arm"].Color = t74.RightArm
        obj4["Left Arm"].Color = t74.LeftArm
        obj4["Right Leg"].Color = t74.RightLeg
        obj4["Left Leg"].Color = t74.LeftLeg
        obj4.Head.Color = t74.Head
        obj4.Torso.Color = t74.Torso

        for _, v82 in ipairs(t75) do
            local obj7 = v82:Clone()

            if obj7:IsA("Accessory") and obj7:FindFirstChild("AccessoryWeld") then
                local name = obj7.AccessoryWeld.Part1.Name
                obj7.AccessoryWeld.Part0 = obj7.Handle

                if obj4:FindFirstChild(name) then
                    obj7.AccessoryWeld.Part1 = obj4[name]
                end
            end

            obj7.Parent = obj4
        end

        if face and face.Parent then
            face:Clone().Parent = obj4.Head
        end

        for v83, v84 in pairs(t73) do
            if not (v84 and v84.Parent) then
                continue
            end

            local v85 = obj4:FindFirstChild(v83 == "RightArm" and "Right Arm" or (v83 == "LeftArm" and "Left Arm" or (v83 == "RightLeg" and "Right Leg" or (v83 == "LeftLeg" and "Left Leg" or v83))))

            if v85 then
                v84:Clone().Parent = v85
            end
        end
    end
end

players.LocalPlayer.CharacterAppearanceLoaded:Connect(fn_u86)

local function fn_u103(p87, p88)
    local world = p88.World

    for _, v89 in ipairs(world:GetChildren()) do
        v89:Destroy()
    end

    local v90 = v_u17:Clone()
    v90.Parent = world
    local instance2 = Instance.new("Camera")
    instance2.CameraType = Enum.CameraType.Scriptable
    instance2.CFrame = (v90.HumanoidRootPart.CFrame + v90.HumanoidRootPart.CFrame.LookVector * 4.5) * CFrame.Angles(0, 3.141592653589793, 0)
    p88.CurrentCamera = instance2
    local v91 = t_u33[p87]

    if not v91 then
        return
    end

    local instance3 = Instance.new("Animation")
    instance3.AnimationId = v91
    local v92 = v90.Humanoid.Animator:LoadAnimation(instance3)
    instance3:Destroy()
    v92.Looped = true
    v92.Priority = Enum.AnimationPriority.Action4

    if p87 == "Popcorn" then
        local instance4 = Instance.new("Motor6D")
        instance4.Name = "Popcorn"
        instance4.Part0 = v90["Right Arm"]
        instance4.C0 = CFrame.new(-0.16, -1.17, -0.67, -1, 0, 0, 0, 0, -1, 0, -1, 0) * CFrame.Angles(0, 3.141592653589793, 0)
        instance4.Parent = v90["Right Arm"]
        local popcorn = emotes:WaitForChild("Popcorn"):Clone()
        popcorn.Parent = v90
        instance4.Part1 = popcorn
    elseif p87 == "Soda" then
        local instance5 = Instance.new("Motor6D")
        instance5.Name = "Cola"
        instance5.Part0 = v90["Right Arm"]
        instance5.C0 = CFrame.new(0.01, -0.91, -0.13, 1, 0, 0, 0, 0, 1, 0, -1, 0)
        instance5.C1 = CFrame.new(0, 0, 0.1)
        instance5.Parent = v90["Right Arm"]
        local cola = emotes.Cola:Clone()
        cola.Parent = v90
        instance5.Part1 = cola
    elseif p87 == "Burger" then
        local burgerModel = emotes.BurgerModel:Clone()
        burgerModel.Burger.Transparency = 0
        burgerModel.BurgerMain.ParentPart.Part0 = v90.HumanoidRootPart
        burgerModel.Parent = v90
    elseif p87 == "Chair" then
        local chair = emotes.Chair:Clone()
        chair.PlasticChair.Transparency = 0
        chair.ChairMain.ParentPart.Part0 = v90.HumanoidRootPart
        chair.Parent = v90
    elseif p87 == "Baja Tragedy" then
        local baja = emotes.Baja:Clone()
        baja.Liquid.Transparency = 0.1
        baja.Spill.Transparency = 0.1
        baja.Cup.Transparency = 0.5
        baja.MainPartMotor.Part0 = v90.HumanoidRootPart
        baja.SpillMotor.Part0 = v90.HumanoidRootPart
        baja.Parent = v90
    elseif p87 == "Donuts" then
        local donuts = emotes.Donuts:Clone()

        for _, v93 in ipairs(donuts:GetDescendants()) do
            if v93:IsA("BasePart") and v93.Name ~= "MainPart" then
                v93.Transparency = 0
            end
        end

        donuts.Motor.Part0 = v90.HumanoidRootPart
        donuts.Parent = v90
    elseif p87 == "Selfie" then
        local phone = emotes.Phone:Clone()
        phone.PhoneMain.ParentPart.Part0 = v90["Right Arm"]
        phone.Parent = v90
    elseif p87 == "Money Walk" then
        local handle = emotes.MoneyBag.Handle:Clone()
        handle.Handle.Part0 = v90["Right Arm"]
        handle.Parent = v90
    elseif p87 == "Minecart" then
        local mineCart = emotes.MineCart:Clone()
        mineCart.MineCart.Part0 = v90.HumanoidRootPart
        mineCart.Parent = v90
    elseif p87 == "Atomic Bomb" or p87 == "Obliteration" then
        local instance6 = Instance.new("Motor6D")
        instance6.Name = "SwordHandle"
        instance6.Part0 = v90["Right Arm"]
        instance6.C0 = CFrame.new(-0.1, -1.03, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0)
        instance6.C1 = CFrame.new(0, 1.5, 0)
        instance6.Parent = v90["Right Arm"]
        local swordHandle = emotes:WaitForChild("SwordHandle"):Clone()
        swordHandle.Parent = v90
        instance6.Part1 = swordHandle
    elseif p87 == "Disgraced Stance" or (p87 == "Vessel Stance" or p87 == "Honored Stance") then
        local energyStanceRightFingers = emotes.EnergyStanceRightFingers:Clone()
        energyStanceRightFingers.Motor.Part0 = v90["Right Arm"]
        local color = v90["Right Arm"].Color

        for _, v94 in ipairs(energyStanceRightFingers:GetChildren()) do
            if v94:IsA("MeshPart") then
                v94.Color = color
            end
        end

        energyStanceRightFingers.Parent = v90
        local energyStanceLeftFingers = emotes.EnergyStanceLeftFingers:Clone()
        energyStanceLeftFingers.Motor.Part0 = v90["Left Arm"]
        local color2 = v90["Left Arm"].Color

        for _, v95 in ipairs(energyStanceLeftFingers:GetChildren()) do
            if v95:IsA("MeshPart") then
                v95.Color = color2
            end
        end

        energyStanceLeftFingers.Parent = v90
    elseif p87 == "Nuh-Uh" or p87 == "Snap" then
        local rightFingers = emotes.RightFingers:Clone()
        rightFingers.Motor.Part0 = v90["Right Arm"]
        local color3 = v90["Right Arm"].Color

        for _, v96 in ipairs(rightFingers:GetChildren()) do
            if v96:IsA("MeshPart") then
                v96.Color = color3
            end
        end

        rightFingers.Parent = v90
    elseif p87 == "Bright Idea" then
        local leftFingers = emotes.LeftFingers:Clone()
        leftFingers.Motor.Part0 = v90["Left Arm"]
        local color4 = v90["Left Arm"].Color

        for _, v97 in ipairs(leftFingers:GetChildren()) do
            if v97:IsA("MeshPart") then
                v97.Color = color4
            end
        end

        leftFingers.Parent = v90
    elseif p87 == "Rock \'n\' Roll" then
        local t98 = {
            name = "Circle",
            c0 = nil,
            c1 = nil
        }
        t98.c0 = CFrame.new(0.04, -0.71, -1.73)
        t98.c1 = CFrame.new(0.06, 0, 0)
        local t99 = {
            name = "Circle.001",
            c0 = nil,
            c1 = nil
        }
        t99.c0 = CFrame.new(-4.38, -0.17, 2.87)
        t99.c1 = CFrame.new(0, 0.061, 0)
        local t100 = {
            name = "Circle.002",
            c0 = nil,
            c1 = nil
        }
        t100.c0 = CFrame.new(4.47, -0.17, 2.87)
        t100.c1 = CFrame.new(0, 0.061, 0)

        for _, v101 in ipairs({ t98, t99, t100 }) do
            local instance7 = Instance.new("Motor6D")
            instance7.Name = v101.name
            instance7.Part0 = v90.HumanoidRootPart
            instance7.C0 = v101.c0
            instance7.C1 = v101.c1
            instance7.Parent = v90.HumanoidRootPart
            local v102 = emotes.RocknRoll[v101.name]:Clone()
            v102.Parent = v90
            instance7.Part1 = v102
        end
    end

    v92:Play(0)
end

instance.Event:Connect(function(p104)
    if p104.Request == "ViewportAnimation" then
        fn_u103(p104.EmoteName, p104.Slot)
    end
end)

local function fn_u122()
    while true do
        local v105 = obj

        if typeof(v105) == "number" then
            break
        end

        task.wait()
    end

    while true do
        local v106 = obj2

        if typeof(v106) == "table" then
            break
        end

        task.wait()
    end

    while true do
        local v107 = s_u13

        if typeof(v107) == "table" then
            break
        end

        task.wait()
    end

    while true do
        local v108 = s_u14

        if typeof(v108) == "table" then
            break
        end

        task.wait()
    end

    while true do
        local v109 = v_u22

        if typeof(v109) == "boolean" then
            break
        end

        task.wait()
    end

    local v110 = obj
    local v111 = obj2["Page" .. tostring(v110)]

    if typeof(v111) == "table" then
        for v112, v113 in pairs(v111) do
            if v112 == "Name" then
                wheel.PageName.Text = v113
            else
                local num = tonumber(v112)

                if not (num and t_u1[num]) then
                    continue
                end

                local v114 = t_u1[num]
                local str = tostring(v113)
                local v115 = s_u13[str]

                if typeof(v115) == "string" then
                    local v116 = s_u13[str]
                    local v117 = s_u14[str][1]
                    local v118 = s_u14[str][2]
                    v114.Text.Text = v116
                    local subtext = v114.Subtext
                    local v119 = table.find(t_u25, subtext)

                    if v117 == "Kill Emote" then
                        if not v119 then
                            local v120 = t_u25
                            -- aliased fastcall table.insert (called via local/upvalue)
                            table.insert(v120, subtext)
                        end
                    elseif v119 then
                        table.remove(t_u25, v119)
                    end

                    subtext.Text = v117
                    subtext.TextColor3 = v118 or v_u27
                    instance:Fire({
                        Request = "ViewportAnimation",
                        EmoteName = v116,
                        Slot = v114
                    })
                else
                    v114.Text.Text = "None"
                    local subtext2 = v114.Subtext
                    local v121 = table.find(t_u25, subtext2)

                    if v121 then
                        table.remove(t_u25, v121)
                    end

                    subtext2.Text = ""
                    instance:Fire({
                        Request = "ViewportAnimation",
                        EmoteName = "None",
                        Slot = v114
                    })
                end
            end
        end
    end
end

local function fn_u127()
    for _, v123 in ipairs(list2:GetChildren()) do
        if not v123:IsA("Frame") or v123:GetAttribute("o") then
            continue
        end

        local v124 = v123:GetAttribute("ENUM")

        if typeof(v124) ~= "string" or not table.find(t_u20, v124) then
            continue
        end

        v123:SetAttribute("o", true)
        local purchase = v123.Purchase
        tweenService:Create(purchase, tweenInfo, {
            BackgroundTransparency = 0.35
        }):Play()
        local price = purchase.Price
        local v125 = table.find(t_u26, price)

        if v125 then
            table.remove(t_u26, v125)
        end

        price:SetAttribute("NormalPrice", nil)
        price.Text = "[OWNED]"
        price.UIStroke.Thickness = 1
        price.UIStroke.Color = v_u30
        local instance8 = Instance.new("Frame")
        instance8.Size = uDim2
        instance8.BorderSizePixel = 0
        instance8.BackgroundColor3 = v_u27
        local instance9 = Instance.new("UICorner")
        instance9.CornerRadius = uDim
        instance9.Parent = instance8
        local v126 = tweenService:Create(instance8, tweenInfo2, {
            BackgroundTransparency = 1
        })
        v126.Completed:Once(function()
            if instance8 and instance8.Parent then
                instance8:Destroy()
            end
        end)
        instance8.Parent = purchase
        v126:Play()
    end
end

local function fn_u143(p_u128, p_u129, p_u130, p_u131)
    local v132 = listButton:Clone()
    local backgroundTransparency = v132.BackgroundTransparency
    local v_u133 = tweenService:Create(v132, tweenInfo, {
        BackgroundTransparency = 0.2
    })
    local t134 = {
        BackgroundTransparency = nil
    }
    t134.BackgroundTransparency = backgroundTransparency
    local v_u135 = tweenService:Create(v132, tweenInfo, t134)
    v132.Button.MouseEnter:Connect(function()
        v_u133:Play()
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIButtonHover"
        })
    end)
    v132.Button.MouseLeave:Connect(function()
        v_u135:Play()
    end)
    v132.Button.MouseButton1Click:Connect(function()
        if not v_u21 then
            return
        end

        local v136 = obj
        local v137 = "Page" .. tostring(v136)
        local v138 = obj2[v137]

        if typeof(v138) == "table" then
            for i = 1, 8 do
                if v_u21 == t_u1[i] then
                    obj2[v137][tostring(i)] = p_u128
                    emote:FireServer({
                        Request = "ChangeSlot",
                        Page = obj,
                        Slot = i,
                        Enumeration = p_u128
                    })
                    break
                end
            end
        end

        v_u21.Text.Text = p_u129
        local subtext3 = v_u21.Subtext
        local v139 = table.find(t_u25, subtext3)

        if p_u130 == "Kill Emote" then
            if not v139 then
                local v140 = t_u25
                -- aliased fastcall table.insert (called via local/upvalue)
                table.insert(v140, subtext3)
            end
        elseif v139 then
            table.remove(t_u25, v139)
        end

        subtext3.Text = p_u130
        subtext3.TextColor3 = p_u131 or v_u27
        instance:Fire({
            Request = "ViewportAnimation",
            EmoteName = p_u129,
            Slot = v_u21
        })
        v_u21 = nil
        v_u67:Cancel()
        v_u68:Cancel()
        v_u71:Cancel()
        fn_u66()
        v_u71:Play()
        list.Visible = false
        list.Search.Text = ""
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIWhoosh"
        })
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIButtonSelect"
        })
    end)
    v132.Text.Text = p_u129
    v132.Parent = list.ScrollingFrame
    local v141 = instance
    local t142 = {
        Request = "ViewportAnimation",
        EmoteName = nil,
        Slot = nil
    }
    t142.EmoteName = p_u129
    t142.Slot = v132.Viewport
    v141:Fire(t142)
    return v132
end

local function fn169()
    local v144 = workspace:GetAttribute("LimitedShopCycle")

    if typeof(v144) ~= "number" then
        return
    end

    for _, v145 in ipairs(list2:GetChildren()) do
        if not v145:IsA("Frame") then
            continue
        end

        local purchase2 = v145.Purchase
        local uITextSizeConstraint2 = purchase2.EmoteName.UITextSizeConstraint
        local price2 = purchase2.Price
        local uITextSizeConstraint3 = price2.UITextSizeConstraint
        local v146 = table.find(t_u24, uITextSizeConstraint2)

        if v146 then
            table.remove(t_u24, v146)
        end

        local v147 = table.find(t_u24, uITextSizeConstraint3)

        if v147 then
            table.remove(t_u24, v147)
        end

        local v148 = table.find(t_u26, price2)

        if v148 then
            table.remove(t_u26, v148)
        end

        v145:Destroy()
    end

    local v149 = limitedShop[v144]

    if not v149 then
        return
    end

    local emotes2 = v149.Emotes

    if typeof(emotes2) == "table" and #v149.Emotes > 0 then
        for _, v150 in ipairs(v149.Emotes) do
            local display = v150.Display
            local enumeration = v150.Enumeration
            local icon = v150.Icon
            local video = v150.Video
            local v_u151 = v150.VideoVolume or 5
            local v_u152 = v_u19:Clone()
            v_u152:SetAttribute("ENUM", enumeration)
            v_u152.Name = display
            local purchase3 = v_u152.Purchase
            local emoteName = purchase3.EmoteName
            emoteName.Text = display
            purchase3.Icon.Image = "rbxassetid://" .. tostring(icon)
            local v153 = t_u24
            local uITextSizeConstraint4 = emoteName.UITextSizeConstraint
            -- aliased fastcall table.insert (called via local/upvalue)
            table.insert(v153, uITextSizeConstraint4)
            local price3 = purchase3.Price
            local v154 = t_u24
            local uITextSizeConstraint5 = price3.UITextSizeConstraint
            -- aliased fastcall table.insert (called via local/upvalue)
            table.insert(v154, uITextSizeConstraint5)

            if table.find(t_u20, enumeration) then
                v_u152:SetAttribute("o", true)
                purchase3.BackgroundTransparency = 0.35
                price3.Text = "[OWNED]"
                price3.UIStroke.Thickness = 1
                price3.UIStroke.Color = v_u30
            else
                local price4 = v150.Price
                local v155

                if localPlayer:GetAttribute("OwnsVIP") then
                    local v156 = price4 * 0.85
                    v155 = math.floor(v156)
                else
                    v155 = price4
                end

                price3:SetAttribute("NormalPrice", price4)
                local v157

                if typeof(v155) == "number" then
                    v157 = tostring(v155):reverse():gsub("%d%d%d", "%1,"):reverse():gsub("^,", "")
                else
                    v157 = nil
                end

                price3.Text = v157 .. " GOLD"
                local v158 = t_u26
                -- aliased fastcall table.insert (called via local/upvalue)
                table.insert(v158, price3)
            end

            local v_u159 = tweenService:Create(purchase3, tweenInfo, {
                BackgroundTransparency = 0.5
            })
            local v_u160 = tweenService:Create(purchase3, tweenInfo, {
                BackgroundTransparency = 0.8
            })
            purchase3.MouseEnter:Connect(function()
                if not v_u152:GetAttribute("o") and v_u159 then
                    v_u159:Play()
                end

                clientCommunication:Fire({
                    Request = "PlaySound",
                    SoundName = "UIButtonHover"
                })
            end)
            purchase3.MouseLeave:Connect(function()
                if not v_u152:GetAttribute("o") and v_u160 then
                    v_u160:Play()
                end
            end)
            purchase3.MouseButton1Click:Connect(function()
                if table.find(t_u20, enumeration) then
                    uIService.Notify("You already own that emote!", nil, nil, 129460368997936)
                else
                    local v161 = localPlayer:GetAttribute("LocalGold")
                    local v162 = price3:GetAttribute("NormalPrice")

                    if typeof(v161) == "number" and typeof(v162) == "number" then
                        local v163

                        if localPlayer:GetAttribute("OwnsVIP") then
                            local v164 = v162 * 0.85
                            v163 = math.floor(v164)
                        else
                            v163 = v162
                        end

                        if v163 <= v161 then
                            limitedShop2:FireServer({
                                Request = "BuyEmote",
                                E = nil,
                                E = enumeration
                            })
                        else
                            local v165 = v163 - v161

                            if v165 <= 500 then
                                buyGold:FireServer({
                                    Request = "Buy500"
                                })
                            elseif v165 <= 1000 then
                                buyGold:FireServer({
                                    Request = "Buy1000"
                                })
                            elseif v165 <= 2500 then
                                buyGold:FireServer({
                                    Request = "Buy2500"
                                })
                            else
                                buyGold:FireServer({
                                    Request = "Buy5000"
                                })
                            end
                        end
                    end
                end

                clientCommunication:Fire({
                    Request = "PlaySound",
                    SoundName = "UIButtonClick"
                })
            end)

            if typeof(video) == "number" then
                local v_u166 = "rbxassetid://" .. video
                local preview2 = v_u152.Preview.Preview
                local v_u167 = tweenService:Create(preview2, tweenInfo, {
                    ImageColor3 = nil,
                    ImageColor3 = v_u28
                })
                local v_u168 = tweenService:Create(preview2, tweenInfo, {
                    ImageColor3 = nil,
                    ImageColor3 = v_u27
                })
                preview2.MouseEnter:Connect(function()
                    clientCommunication:Fire({
                        Request = "PlaySound",
                        SoundName = "UIButtonHover"
                    })
                    v_u167:Play()
                end)
                preview2.MouseLeave:Connect(function()
                    v_u168:Play()
                end)
                preview2.MouseButton1Click:Connect(function()
                    wheel.Visible = false
                    preview.TimePosition = 0
                    preview.Playing = true
                    preview.Video = v_u166
                    preview.Volume = v_u151
                    videoPlayer.Visible = true
                    clientCommunication:Fire({
                        Request = "PlaySound",
                        SoundName = "UIButtonClick"
                    })
                end)
            else
                v_u152.Preview:Destroy()
            end

            v_u152.Parent = list2
        end

        fn_u62()
        return
    end
end

local v170 = workspace
local s_u171 = "LimitedShopCycle"
local v172 = v170:GetAttributeChangedSignal(s_u171)
s_u171 = fn169
v172:Connect(s_u171)
s_u171 = close
local v_u173 = tweenService:Create(s_u171, tweenInfo, {
    TextColor3 = v_u28
})
n_u70 = tweenService:Create(close, tweenInfo, {
    TextColor3 = v_u27
})
s_u171 = close.MouseEnter
s_u171:Connect(function()
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonHover"
    })
    v_u173:Play()
end)
s_u171 = close.MouseLeave
s_u171:Connect(function()
    n_u70:Play()
end)
s_u171 = close.MouseButton1Click
s_u171:Connect(function()
    videoPlayer.Visible = false
    preview.Playing = false
    preview.Video = ""
    wheel.Visible = true
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonClick"
    })
end)
local onClientEvent = emote.OnClientEvent
s_u171 = function(p174)
    local request = p174.Request

    if request == "NewHumanoidDescription" then
        fn_u86(players.LocalPlayer.Character)
        return
    end

    if request == "OwnsAllEmotes" then
        uIService.Notify("You already own every emote!", nil, nil, 129460368997936)
        return
    end

    if request == "BoughtEmote" then
        uIService.Notify("Unlocked new emote!", nil, nil, 129460368997936)
        return
    end

    if request == "BoughtLimEmote" then
        uIService.Notify("Bought limited time emote!", nil, nil, 129460368997936)
        return
    end

    if request == "NotEnoughGold" then
        uIService.Notify("You don\'t have enough Gold!", nil, nil, 129460368997936)
        return
    end

    if request == "BoughtGamePass" then
        v_u22 = true

        if wheel:FindFirstChild("GamePass") then
            wheel.GamePass:Destroy()
        end

        if not v_u21 then
            t_u1[5].Visible = true
            t_u1[6].Visible = true
            t_u1[7].Visible = true
            t_u1[8].Visible = true
        end

        if p174.N ~= true then
            uIService.Notify("Successfully purchased GamePass!", nil, nil, 129460368997936)
            return
        end
    else
        if request == "UpdateVIPPrices" then
            if discount and discount.Parent then
                discount:Destroy()
            end

            price1.Text = "425 Gold"

            for _, v175 in ipairs(t_u26) do
                if not (v175 and v175.Parent) then
                    continue
                end

                local v176 = v175:GetAttribute("NormalPrice")

                if typeof(v176) ~= "number" then
                    continue
                end

                local v177 = v176 * 0.85
                local v178 = math.floor(v177)
                local v179

                if typeof(v178) == "number" then
                    v179 = tostring(v178):reverse():gsub("%d%d%d", "%1,"):reverse():gsub("^,", "")
                else
                    v179 = nil
                end

                v175.Text = v179 .. " GOLD"
            end

            return
        end

        if request == "BoughtPage" then
            while true do
                local v180 = obj2

                if typeof(v180) == "table" then
                    break
                end

                task.wait()
            end

            local number = p174.Number
            obj2["Page" .. tostring(number)] = {
                ["1"] = 0,
                ["2"] = 0,
                ["3"] = 0,
                ["4"] = 0,
                ["5"] = 0,
                ["6"] = 0,
                ["7"] = 0,
                ["8"] = 0,
                Name = "Page " .. tostring(number)
            }

            if p174.N ~= true then
                uIService.Notify("Successfully purchased page!", nil, nil, 129460368997936)
                return
            end
        else
            if request == "AddEmoteToList" then
                local enumeration2 = p174.Enumeration
                local str2 = tostring(enumeration2)
                local v181 = t_u20
                -- aliased fastcall table.insert (called via local/upvalue)
                table.insert(v181, str2)
                fn_u127()
                fn_u143(enumeration2, s_u13[str2], s_u14[str2][1], s_u14[str2][2])
                return
            end

            if request ~= "Initialize" then
                return
            end

            local page = p174.Page
            local pages = p174.Pages
            local owns = p174.Owns
            local enumerations = p174.Enumerations
            local types = p174.Types
            local owned = p174.Owned
            local gold = p174.Gold

            if typeof(gold) == "number" and typeof(gold) == "number" then
                localPlayer:SetAttribute("LocalGold", gold)
                local s182 = "Balance: "
                local v183

                if typeof(gold) == "number" then
                    v183 = tostring(gold):reverse():gsub("%d%d%d", "%1,"):reverse():gsub("^,", "")
                else
                    v183 = nil
                end

                local v184 = s182 .. v183 .. " Gold"

                for _, v185 in ipairs(t_u15) do
                    v185.Text = v184
                end
            end

            if typeof(page) == "number" then
                obj = page
            end

            if typeof(pages) == "table" then
                obj2 = pages
            end

            if typeof(enumerations) == "table" then
                s_u13 = enumerations
            end

            if typeof(types) == "table" then
                s_u14 = types
            end

            if typeof(owns) == "boolean" then
                v_u22 = owns
            end

            for i = 1, math.huge do
                local v186 = owned[tostring(i)]

                if typeof(v186) ~= "number" then
                    break
                end

                local str3 = tostring(v186)
                local v187 = enumerations[str3]
                local v188 = types[str3][1]
                local v189 = types[str3][2]

                if typeof(v187) ~= "string" then
                    break
                end

                local v190 = t_u20
                -- aliased fastcall table.insert (called via local/upvalue)
                table.insert(v190, str3)
                fn_u143(v186, v187, v188, v189)
            end

            fn_u122()

            if owns then
                if wheel:FindFirstChild("GamePass") then
                    wheel.GamePass:Destroy()
                end

                t_u1[5].Visible = true
                t_u1[6].Visible = true
                t_u1[7].Visible = true
                t_u1[8].Visible = true
            end

            fn_u62()
            fn_u127()
        end
    end
end
onClientEvent:Connect(s_u171)
local onClientEvent2 = buyGold.OnClientEvent
s_u171 = function(p191)
    if p191.Request ~= "Update" then
        return
    end

    local new2 = p191.New

    if typeof(new2) ~= "number" then
        return
    end

    localPlayer:SetAttribute("LocalGold", new2)
    local s192 = "Balance: "
    local v193

    if typeof(new2) == "number" then
        v193 = tostring(new2):reverse():gsub("%d%d%d", "%1,"):reverse():gsub("^,", "")
    else
        v193 = nil
    end

    local v194 = s192 .. v193 .. " Gold"

    for _, v195 in ipairs(t_u15) do
        v195.Text = v194
    end
end
onClientEvent2:Connect(s_u171)

local function fn204(p_u196)
    local button2 = frame:WaitForChild((tostring(p_u196))):WaitForChild("Button")
    local t197 = {
        BackgroundColor3 = nil
    }
    t197.BackgroundColor3 = v_u31
    local v_u198 = tweenService:Create(button2, tweenInfo, t197)
    local t199 = {
        BackgroundColor3 = nil
    }
    t199.BackgroundColor3 = v_u27
    local v_u200 = tweenService:Create(button2, tweenInfo, t199)
    button2.MouseEnter:Connect(function()
        v_u198:Play()
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIButtonHover"
        })
    end)
    button2.MouseLeave:Connect(function()
        v_u200:Play()
    end)
    button2.MouseButton1Click:Connect(function()
        local v201 = buyGold
        local t202 = {
            Request = nil
        }
        local v203 = p_u196
        t202.Request = "Buy" .. tostring(v203)
        v201:FireServer(t202)
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIButtonSelect"
        })
    end)
end

n_u70 = fn204
s_u171 = 500
n_u70(s_u171)
n_u70 = fn204
s_u171 = 1000
n_u70(s_u171)
n_u70 = fn204
s_u171 = 2500
n_u70(s_u171)
n_u70 = fn204
s_u171 = 5000
n_u70(s_u171)
n_u70 = discount and discount.Parent
local v_u205

if n_u70 then
    n_u70 = tweenService:Create(discount, tweenInfo, {
        ImageColor3 = v32
    })
    v_u205 = {
        ImageColor3 = nil
    }
    v_u205.ImageColor3 = v_u27
    s_u171 = tweenService:Create(discount, tweenInfo, v_u205)
    discount.MouseEnter:Connect(function()
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIButtonHover"
        })
        n_u70:Play()
    end)
    discount.MouseLeave:Connect(function()
        s_u171:Play()
    end)
    discount.MouseButton1Click:Connect(function()
        vIP:FireServer({
            Request = "BuyGamePass"
        })
        clientCommunication:Fire({
            Request = "PlaySound",
            SoundName = "UIButtonClick"
        })
    end)
end

n_u70 = nil
s_u171 = nil
local overlapParams = OverlapParams.new()
overlapParams.FilterDescendantsInstances = { workspace:WaitForChild("Characters") }
overlapParams.FilterType = Enum.RaycastFilterType.Include

local function fn_u212()
    n_u70 = task.spawn(function()
        local v206 = 604800 - (workspace:GetServerTimeNow() - 1746187200) % 604800
        leaving.Text = "Leaving in " .. fn_u55(v206)
        task.wait(v206 - math.floor(v206))
        leaving.Text = "Leaving in " .. fn_u55(604800 - (workspace:GetServerTimeNow() - 1746187200) % 604800)

        while task.wait(1) do
            leaving.Text = "Leaving in " .. fn_u55(604800 - (workspace:GetServerTimeNow() - 1746187200) % 604800)
        end
    end)
    s_u171 = task.spawn(function()
        while true do
            while #t_u25 <= 0 do
                task.wait()
            end

            local character = localPlayer.Character
            local b207 = false

            if character and character.Parent then
                local instance10 = Instance.new("Part")
                instance10.Transparency = 1
                instance10.Size = Vector3.new(10, 9, 10)
                instance10.CFrame = character.HumanoidRootPart.CFrame
                instance10.Massless = true
                instance10.EnableFluidForces = false
                instance10.CastShadow = false
                instance10.CanCollide = false
                instance10.Anchored = true
                instance10.Parent = vFX
                local v208 = workspace:GetPartsInPart(instance10, overlapParams)
                instance10:Destroy()

                for _, v209 in ipairs(v208) do
                    if v209.Name ~= "HumanoidRootPart" or v209:IsDescendantOf(character) then
                        continue
                    end

                    local parent = v209.Parent

                    if not parent or (not parent.Parent or parent:GetAttribute("NoKillEmote")) then
                        continue
                    end

                    local humanoid = parent:FindFirstChild("Humanoid")

                    if humanoid and (humanoid.Parent and humanoid.Health <= 0) then
                        b207 = true
                    end
                end
            end

            local v210 = b207 and "USE!" or "Kill Emote"

            for _, v211 in ipairs(t_u25) do
                v211.Text = v210
            end

            task.wait()
        end
    end)
end

local function fn_u213()
    pcall(function()
        task.cancel(n_u70)
    end)
    pcall(function()
        task.cancel(s_u171)
    end)
end

v_u205 = containerFrame:GetPropertyChangedSignal("Visible")
v_u205:Connect(function()
    if containerFrame.Visible then
        fn_u212()

        if preview.Visible and preview.Video ~= "" then
            preview.Playing = true
            return
        end
    else
        fn_u213()
        preview.Playing = false
    end
end)

while true do
    local v214 = obj
    v_u205 = typeof
    v_u205 = v_u205(v214)

    if v_u205 == "number" then
        break
    end

    v_u205 = task.wait
    v_u205()
end

while true do
    local v215 = obj2
    v_u205 = typeof
    v_u205 = v_u205(v215)

    if v_u205 == "table" then
        break
    end

    v_u205 = task.wait
    v_u205()
end

while true do
    local v216 = s_u13
    v_u205 = typeof
    v_u205 = v_u205(v216)

    if v_u205 == "table" then
        break
    end

    v_u205 = task.wait
    v_u205()
end

while true do
    local v217 = v_u22
    v_u205 = typeof
    v_u205 = v_u205(v217)

    if v_u205 == "boolean" then
        break
    end

    v_u205 = task.wait
    v_u205()
end

v_u205 = ipairs
local v218, v219
v_u205, v218, v219 = v_u205(t2)

for _, obj8 in v_u205, v218, v219 do
    if not (obj8 and obj8.Parent) then
        continue
    end

    obj8.MouseButton1Click:Connect(function()
        if v_u21 then
            return
        end

        local instance11 = Instance.new("Configuration")
        instance11.Name = "CloseSignal"
        instance11.Parent = containerFrame
        instance11:Destroy()
        local text = obj8.Parent.Text.Text

        if text == "None" then
            return
        end

        local v220 = nil

        for v221, v222 in pairs(s_u13) do
            if v222 == text then
                v220 = tonumber(v221)
                break
            end
        end

        if typeof(v220) == "number" and not players.LocalPlayer.Character.HumanoidRootPart:FindFirstChild("DashVelocity") then
            emote:FireServer({
                Request = "PlayEmote",
                Enumeration = v220
            })
        end
    end)
    obj8.MouseButton1Down:Connect(function()
        local v_u223 = runService
        v_u223:UnbindFromRenderStep("SlotButtonHeld")
        v_u223 = 0
        runService:BindToRenderStep("SlotButtonHeld", Enum.RenderPriority.First.Value, function(p224)
            if v_u21 then
                runService:UnbindFromRenderStep("SlotButtonHeld")
            else
                v_u223 = v_u223 + p224

                if v_u223 < 0.435 then
                    return
                end

                v_u21 = obj8.Parent
                fn_u65()
                v_u67:Cancel()
                v_u68:Cancel()
                v_u71:Cancel()
                wheel.Position = UDim2.new(0.5, 0, 0.5, 0)
                list.Position = UDim2.new(0.6, 0, 0.5, 0)
                v_u67:Play()
                v_u68:Play()
                list.Visible = true
                clientCommunication:Fire({
                    Request = "PlaySound",
                    SoundName = "UIWhoosh"
                })
            end
        end)
    end)
    obj8.MouseLeave:Connect(function()
        runService:UnbindFromRenderStep("SlotButtonHeld")
    end)
    obj8.MouseButton1Up:Connect(function()
        runService:UnbindFromRenderStep("SlotButtonHeld")
    end)
end

v_u205 = tweenService:Create(list.Exit, tweenInfo, {
    ImageColor3 = Color3.fromRGB(255, 0, 0)
})
local v_u225 = tweenService:Create(list.Exit, tweenInfo, {
    ImageColor3 = v_u27
})
list.Exit.MouseEnter:Connect(function()
    v_u205:Play()
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonHover"
    })
end)
list.Exit.MouseLeave:Connect(function()
    v_u225:Play()
end)
list.Exit.MouseButton1Click:Connect(function()
    v_u21 = nil
    v_u67:Cancel()
    v_u68:Cancel()
    v_u71:Cancel()
    fn_u66()
    v_u71:Play()
    list.Visible = false
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIWhoosh"
    })
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonClick"
    })
end)
v_u205 = list.Search
v_u205 = v_u205:GetPropertyChangedSignal("Text")
v_u205:Connect(function()
    local v226 = string.lower(list.Search.Text)

    if v226 == "" then
        for _, v227 in ipairs(list.ScrollingFrame:GetChildren()) do
            if v227:IsA("Frame") then
                v227.Visible = true
            end
        end
    else
        for _, v228 in ipairs(list.ScrollingFrame:GetChildren()) do
            if not v228:IsA("Frame") then
                continue
            end

            local v229 = string.lower(v228.Text.Text)
            v228.Visible = string.match(v229, v226) ~= nil
        end
    end
end)
v_u205 = nil

local function fn_u242(p230)
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonSelect"
    })

    if v_u21 then
        return
    end

    local v231 = obj
    local n232 = 0
    local v233 = n232

    for i = 1, math.huge do
        local v234 = obj2["Page" .. tostring(i)]

        if typeof(v234) ~= "table" then
            break
        end

        n232 = v233 + 1
        v233 = n232
    end

    if v_u205 then
        v_u205:Disconnect()
        v_u205 = nil
    end

    if p230 == "left" then
        obj = obj - 1 < 1 and v233 and v233 or obj - 1
    else
        obj = v233 < obj + 1 and 1 or obj + 1
    end

    local v235 = obj
    wheel.PageName.Text = obj2["Page" .. tostring(v235)].Name

    if v231 ~= obj then
        fn_u122()
        emote:FireServer({
            Request = "ChangePage",
            New = obj
        })
    end

    v_u205 = wheel.PageName:GetPropertyChangedSignal("Text"):Connect(function()
        local text2 = wheel.PageName.Text

        if #text2 > 30 then
            local v236 = obj
            wheel.PageName.Text = obj2["Page" .. tostring(v236)].Name
        else
            local v237 = obj
            obj2["Page" .. tostring(v237)].Name = text2
            local v238 = obj
            wheel.PageName.Text = obj2["Page" .. tostring(v238)].Name
            local v239 = emote
            local t240 = {
                Request = "ChangePageName",
                New = nil,
                Page = nil
            }
            local v241 = obj
            t240.New = obj2["Page" .. tostring(v241)].Name
            t240.Page = obj
            v239:FireServer(t240)
        end
    end)
end

wheel.Arrows.Left.MouseButton1Click:Connect(function()
    fn_u242("left")
end)
wheel.Arrows.Right.MouseButton1Click:Connect(function()
    fn_u242("right")
end)
local v_u243 = tweenService:Create(wheel.BuyPage, tweenInfo, {
    ImageColor3 = v29
})
local v_u244 = tweenService:Create(wheel.BuyPage.TextLabel, tweenInfo, {
    TextColor3 = v29
})
local v_u245 = tweenService:Create(wheel.BuyPage, tweenInfo, {
    ImageColor3 = v_u27
})
local v_u246 = tweenService:Create(wheel.BuyPage.TextLabel, tweenInfo, {
    TextColor3 = v_u27
})
wheel.BuyPage.MouseEnter:Connect(function()
    v_u243:Play()
    v_u244:Play()
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonHover"
    })
end)
wheel.BuyPage.MouseLeave:Connect(function()
    v_u245:Play()
    v_u246:Play()
end)
wheel.BuyPage.MouseButton1Click:Connect(function()
    emote:FireServer({
        Request = "BuyPage"
    })
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonSelect"
    })
end)
local v_u247 = tweenService:Create(wheel.Purchase, tweenInfo, {
    ImageColor3 = v29
})
local v_u248 = tweenService:Create(wheel.Purchase.Main, tweenInfo, {
    TextColor3 = v29
})
local v_u249 = tweenService:Create(wheel.Purchase, tweenInfo, {
    ImageColor3 = v_u27
})
local v_u250 = tweenService:Create(wheel.Purchase.Main, tweenInfo, {
    TextColor3 = v_u27
})
wheel.Purchase.Button.MouseEnter:Connect(function()
    v_u247:Play()
    v_u248:Play()
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonHover"
    })
end)
wheel.Purchase.Button.MouseLeave:Connect(function()
    v_u249:Play()
    v_u250:Play()
end)
wheel.Purchase.Button.MouseButton1Click:Connect(function()
    emote:FireServer({
        Request = "BuyEmote"
    })
    clientCommunication:Fire({
        Request = "PlaySound",
        SoundName = "UIButtonSelect"
    })
end)

if wheel:FindFirstChild("GamePass") then
    wheel.GamePass.Buy.MouseButton1Click:Connect(function()
        emote:FireServer({
            Request = "BuyGamePass"
        })
    end)
end

v_u205 = wheel.PageName:GetPropertyChangedSignal("Text"):Connect(function()
    local text3 = wheel.PageName.Text

    if #text3 > 30 then
        local v251 = obj
        wheel.PageName.Text = obj2["Page" .. tostring(v251)].Name
    else
        local v252 = obj
        obj2["Page" .. tostring(v252)].Name = text3
        local v253 = obj
        wheel.PageName.Text = obj2["Page" .. tostring(v253)].Name
        local v254 = emote
        local t255 = {
            Request = "ChangePageName",
            New = nil,
            Page = nil
        }
        local v256 = obj
        t255.New = obj2["Page" .. tostring(v256)].Name
        t255.Page = obj
        v254:FireServer(t255)
    end
end)
fn169()
