-- Azeradio addon contract tests.
--
-- Runs with stock Lua (5.1 through 5.4, no dependencies) from the repo root:
--   lua addon/tests/test_core.lua
-- or via the local entry point:
--   .\scripts\test.ps1
--
-- core.lua only talks to the WoW client, so the tests mock the handful of
-- globals it touches, load the real core.lua, then drive it the way the game
-- would: zone/combat events plus /azeradio slash commands. Every assertion
-- pins the bridge-line contract the tray app parses in tray/src/bridge.rs:
--   [AZERADIO] zone="..." subzone="..." instance="..." instanceName="..."
--              instanceID="..." combat="0|1" why="..."
-- If this file and bridge.rs disagree, the game-to-Spotify bridge is broken.

local CORE = arg and arg[1] or "addon/Azeradio/core.lua"

-- ---------------------------------------------------------------- mocks ---

local CAPTURE = { bridge = {}, chat = {} }

local MOCK = {
    zone = "Orgrimmar",
    subzone = "Valley of Honor",
    instanceName = "",
    instanceType = "none",
    instanceID = 0,
    combat = false,
    chatlog = true,
}

local function newFrame(ftype, name)
    local f = { _type = ftype, _name = name, events = {} }
    function f:GetName()
        return self._name or "AzeradioMockFrame"
    end
    function f:SetSize()
    end
    function f:SetPoint()
    end
    function f:SetMaxLines()
    end
    function f:SetFontObject()
    end
    function f:SetClampedToScreen()
    end
    function f:ClearAllPoints()
    end
    function f:RegisterEvent(ev)
        table.insert(self.events, ev)
    end
    function f:SetScript(kind, fn)
        self[kind] = fn
    end
    function f:AddMessage(text)
        table.insert(CAPTURE.bridge, text)
    end
    return f
end

local EVENT_FRAME = nil

-- WoW globals core.lua touches.
NUM_CHAT_WINDOWS = 0
_G.UIParent = {}
_G.ChatFrame1 = {}
_G.ChatFontNormal = {}
_G.CreateFrame = function(ftype, name, parent)
    local f = newFrame(ftype, name)
    if ftype == "Frame" then
        EVENT_FRAME = f
    end
    return f
end
_G.GetZoneText = function()
    return MOCK.zone
end
_G.GetMinimapZoneText = function()
    return MOCK.subzone
end
_G.GetInstanceInfo = function()
    return MOCK.instanceName, MOCK.instanceType, 0, "Normal", 0, 0, false, MOCK.instanceID
end
_G.InCombatLockdown = function()
    return MOCK.combat
end
_G.LoggingChat = function(v)
    if v == nil then
        return MOCK.chatlog
    end
    MOCK.chatlog = (v and true or false)
end
_G.GetCVar = function()
    return MOCK.chatlog and "1" or "0"
end
_G.SetCVar = function(_, v)
    MOCK.chatlog = (v == "1")
end
_G.strtrim = function(s)
    return (s:gsub("^%s+", ""):gsub("%s+$", ""))
end
_G.DEFAULT_CHAT_FRAME = {
    AddMessage = function(_, text)
        table.insert(CAPTURE.chat, text)
    end,
}
_G.SlashCmdList = {}

-- ------------------------------------------------------------ harness ---

local passed, failed = 0, 0

local function check(name, cond, detail)
    if cond then
        passed = passed + 1
        print("  ok: " .. name)
    else
        failed = failed + 1
        print("  FAIL: " .. name .. (detail and (" -- " .. detail) or ""))
    end
end

local function has(haystack, needle)
    return haystack:find(needle, 1, true) ~= nil
end

-- Same accessor the tray uses (bridge.rs kv): value holds no double quote.
local function kv(line, key)
    return line:match(key .. '="([^"]*)"')
end

local function bridgeCount()
    return #CAPTURE.bridge
end

local function fire(event)
    assert(EVENT_FRAME and EVENT_FRAME.OnEvent, "event frame not wired")
    EVENT_FRAME.OnEvent(EVENT_FRAME, event)
end

local function slash(msg)
    assert(SlashCmdList.AZERADIO, "slash command not registered")
    SlashCmdList.AZERADIO(msg)
end

-- ---------------------------------------------------------------- load ---

print("addon contract tests (" .. CORE .. ")")

local chunk, err = loadfile(CORE)
assert(chunk, "cannot load core.lua: " .. tostring(err))
chunk()

check("event frame registered for zone and combat events", (function()
    local want = {
        PLAYER_ENTERING_WORLD = true,
        ZONE_CHANGED_NEW_AREA = true,
        ZONE_CHANGED = true,
        ZONE_CHANGED_INDOORS = true,
        PLAYER_REGEN_DISABLED = true,
        PLAYER_REGEN_ENABLED = true,
    }
    for _, ev in ipairs(EVENT_FRAME.events) do
        want[ev] = nil
    end
    return next(want) == nil
end)(), table.concat(EVENT_FRAME.events, ","))

-- ---------------------------------------------------------------- tests ---

do -- 1: zone change emits one bridge line tagged why="zone"
    local n = bridgeCount()
    fire("ZONE_CHANGED_NEW_AREA")
    check("zone change emits one bridge line", bridgeCount() == n + 1)
    local line = CAPTURE.bridge[#CAPTURE.bridge]
    check("bridge line carries the [AZERADIO] prefix", has(line, "[AZERADIO]"), line)
    check("zone event is tagged why=zone", kv(line, "why") == "zone", line)
end

do -- 2: every key the tray parses is present with the game's values
    fire("ZONE_CHANGED")
    local line = CAPTURE.bridge[#CAPTURE.bridge]
    check("zone value matches GetZoneText", kv(line, "zone") == MOCK.zone, line)
    check("subzone value matches GetMinimapZoneText", kv(line, "subzone") == MOCK.subzone, line)
    check("instance fields present", kv(line, "instance") ~= nil and kv(line, "instanceName") ~= nil and kv(line, "instanceID") ~= nil, line)
    check("combat flag present", kv(line, "combat") == "0", line)
end

do -- 3: instance info travels in instanceName/instanceID
    MOCK.instanceName = "Amirdrassil, the Dream's Hope"
    MOCK.instanceType = "raid"
    MOCK.instanceID = 123
    fire("ZONE_CHANGED_INDOORS")
    local line = CAPTURE.bridge[#CAPTURE.bridge]
    check("instanceName matches GetInstanceInfo", kv(line, "instanceName") == MOCK.instanceName, line)
    check("instanceID matches GetInstanceInfo", kv(line, "instanceID") == "123", line)
    MOCK.instanceName, MOCK.instanceType, MOCK.instanceID = "", "none", 0
end

do -- 4: double quotes in names cannot break the tray's kv parsing
    MOCK.zone = 'A"B'
    fire("ZONE_CHANGED")
    local line = CAPTURE.bridge[#CAPTURE.bridge]
    check("quotes are escaped to single quotes", kv(line, "zone") == "A'B", line)
    check("no raw quote survives inside the value", not has(kv(line, "zone") or "", '"'), line)
    MOCK.zone = "Orgrimmar"
end

do -- 5: combat start/end carry the combat flag and why
    MOCK.combat = true
    fire("PLAYER_REGEN_DISABLED")
    local startLine = CAPTURE.bridge[#CAPTURE.bridge]
    check("combat start sets combat=1", kv(startLine, "combat") == "1", startLine)
    check("combat start is tagged combat_start", kv(startLine, "why") == "combat_start", startLine)
    MOCK.combat = false
    fire("PLAYER_REGEN_ENABLED")
    local endLine = CAPTURE.bridge[#CAPTURE.bridge]
    check("combat end sets combat=0", kv(endLine, "combat") == "0", endLine)
    check("combat end is tagged combat_end", kv(endLine, "why") == "combat_end", endLine)
end

do -- 6: /azeradio combat off silences combat lines, combat on restores them
    slash("combat off")
    MOCK.combat = true
    local n = bridgeCount()
    fire("PLAYER_REGEN_DISABLED")
    check("combat off suppresses combat lines", bridgeCount() == n)
    MOCK.combat = false
    slash("COMBAT ON") -- slash parsing is case-insensitive
    fire("PLAYER_REGEN_ENABLED")
    check("combat on restores combat lines", bridgeCount() == n + 1)
end

do -- 7: /azeradio test emits a bridge line plus a visible confirmation
    local b, c = bridgeCount(), #CAPTURE.chat
    slash("test")
    check("slash test emits a bridge line", bridgeCount() == b + 1)
    check("slash test line is tagged why=test", kv(CAPTURE.bridge[#CAPTURE.bridge], "why") == "test")
    check(
        "slash test confirms in chat",
        #CAPTURE.chat == c + 1 and has(CAPTURE.chat[#CAPTURE.chat], "test event sent"),
        CAPTURE.chat[#CAPTURE.chat]
    )
end

do -- 8: /azeradio log reports and toggles chat logging state
    MOCK.chatlog = true
    slash("log")
    check("slash log reports state", has(CAPTURE.chat[#CAPTURE.chat], "chat logging is on"), CAPTURE.chat[#CAPTURE.chat])
    slash("log off")
    check("slash log off disables logging", MOCK.chatlog == false)
    slash("log on")
    check("slash log on enables logging", MOCK.chatlog == true)
end

do -- 9: entering the world announces like any other zone change
    local n = bridgeCount()
    fire("PLAYER_ENTERING_WORLD")
    check("entering world emits a bridge line", bridgeCount() == n + 1)
    check("entering world is tagged why=zone", kv(CAPTURE.bridge[#CAPTURE.bridge], "why") == "zone")
end

do -- 10: unknown slash input shows usage, bridge lines stay chat-only
    local c = #CAPTURE.chat
    slash("frobnicate")
    check("unknown slash shows usage", #CAPTURE.chat == c + 2 and has(CAPTURE.chat[#CAPTURE.chat], "usage:"), CAPTURE.chat[#CAPTURE.chat])
    local leaked = false
    for _, text in ipairs(CAPTURE.chat) do
        if has(text, 'zone="') then
            leaked = true
        end
    end
    check("bridge lines never leak into chat", not leaked)
end

-- -------------------------------------------------------------- report ---

print(("%d passed, %d failed"):format(passed, failed))
if failed > 0 then
    os.exit(1)
end
