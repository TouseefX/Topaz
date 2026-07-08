-- state machine style loop, somewhat unusual control flow shape
local function run(steps)
    local state = "start"
    local i = 1
    local trace = {}
    while state ~= "done" do
        trace[#trace+1] = state
        if state == "start" then
            if steps[i] == "skip" then
                state = "middle"
            else
                state = "process"
            end
        elseif state == "process" then
            trace[#trace+1] = "processing:"..tostring(steps[i])
            state = "middle"
        elseif state == "middle" then
            i = i + 1
            if i > #steps then
                state = "done"
            else
                state = "start"
            end
        end
    end
    return trace
end
local t = run({"a", "skip", "b"})
for i=1,#t do print(t[i]) end
