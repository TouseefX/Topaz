# Topaz

Topaz is a decompiler for LuaU bytecode that trys to remake a script based off of that bytecode.

All Medal credits to this project goes to in honor and memory of:
Jujhar Singh (KowalskiFX)
Mathias Pedersen (Costomality)

While details of how they passed and their relationship is unknown, it is better if their legacy does not get left forgotten.

Keep the Singh and Pedersen family in your guys' prayers.
We love you both.

## Run Topaz

```lua
local API_URL = "http://localhost:3000/";

local request = (syn and syn.request) or (http and http.request) or http_request or (fluxus and fluxus.request) or request;
local base64Encode = base64_encode or crypt and crypt.base64.encode

getgenv().decompile = function(Script)
    local success, ScriptBytecode = pcall(getscriptbytecode, Script);
    local ScriptBytecode = getscriptbytecode(Script);

    if (not success) then
	    print('Topaz Error: Failed To Get Script bytecode');
	    return `Topaz Decomplier: Failed to Get Bytecode`;
    end

    if (base64Encode) then
        ScriptBytecode = base64Encode(ScriptBytecode);
    else
        print('Topaz Warning: Your LuaU code executor does not support Base64!');
    end

    return request({
        Url = `{API_URL}decompile`;
        Method = "POST";
        Body = ScriptBytecode;
    }).Body;
end;


local synsaveinstance = loadstring(game:HttpGet("https://raw.githubusercontent.com/Devraj2010isme/BetterSaveinstance/saveinstance.luau"))()

local Options = {
  SafeMode = true,
  ShutdownWhenDone = true,
  ShowStatus = true,
  AntiIdle = true,
  timeout = -1,
}
synsaveinstance(Options)
```
