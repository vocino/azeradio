-- Azeradio options-panel tests.
--
-- Runs with stock Lua (no dependencies) from the repo root:
--   lua addon/tests/test_options.lua
--
-- Loads the real core.lua then the real options.lua under mocked WoW
-- globals and drives the panel the way the game would: checkbox clicks,
-- the test button, OnShow refresh, slash commands, and both registration
-- APIs (classic InterfaceOptions_AddCategory and modern Settings).

local CORE = "addon/Azeradio/core.lua"
local OPTIONS = "addon/Azeradio/options.lua"

-- ---------------------------------------------------------------- mocks ---

local CAPTURE = { bridge = {}, chat = {}, sounds = {}, frames = {}, fontstrings = {} }
local REGISTERED = {}

local MOCK = {
    zone = "Orgrimmar",
    subzone = "Valley of Honor",
    instanceName = "",
    instanceType = "none",
    instanceID = 0,
    combat = false,
    chatlog = true,
    drag = false,
}

local TOOLTIP = { owner = nil, text = nil, lines = {}, shown = false }

local function newFontString()
    local fs = { _text = "" }
    function fs:SetPoint()
    end
    function fs:ClearAllPoints()
    end
    function fs:SetJustifyH()
    end
    function fs:SetWidth()
    end
    function fs:SetText(t)
        self._text = t
    end
    table.insert(CAPTURE.fontstrings, fs)
    return fs
end

local EVENT_FRAME = nil

local function newFrame(ftype, name)
    local f = { _type = ftype, _name = name, events = {}, scripts = {}, _checked = false }
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
        self.scripts[kind] = fn
        self[kind] = fn
    end
    function f:AddMessage(text)
        table.insert(CAPTURE.bridge, text)
    end
    function f:CreateFontString(...)
        return newFontString()
    end
    function f:SetChecked(v)
        self._checked = (v and true or false)
    end
    function f:GetChecked()
        return self._checked
    end
    function f:SetText(t)
        self._label = t
    end
    f.Text = {
        SetText = function(_, t)
            f._label = t
        end,
    }
    table.insert(CAPTURE.frames, f)
    if ftype == "Frame" and name == nil then
        EVENT_FRAME = f
    end
    return f
end

-- WoW globals core.lua and options.lua touch.
NUM_CHAT_WINDOWS = 0
_G.UIParent = {}
_G.ChatFrame1 = {}
_G.ChatFontNormal = {}
_G.CreateFrame = function(ftype, name, parent, template)
    return newFrame(ftype, name)
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
_G.PlaySound = function(id)
    table.insert(CAPTURE.sounds, id)
end
_G.SOUNDKIT = {
    IG_MAINMENU_OPTION_CHECKBOX_ON = 856,
    IG_MAINMENU_OPTION_CHECKBOX_OFF = 857,
}
_G.IsMouseButtonDown = function()
    return MOCK.drag
end
_G.GameTooltip = {
    SetOwner = function(_, owner)
        TOOLTIP.owner = owner
    end,
    SetText = function(_, text)
        TOOLTIP.text = text
    end,
    AddLine = function(_, line)
        table.insert(TOOLTIP.lines, line)
    end,
    Show = function()
        TOOLTIP.shown = true
    end,
    Hide = function()
        TOOLTIP.shown = false
    end,
}
_G.InterfaceOptions_AddCategory = function(panel)
    REGISTERED.classic = panel
end
_G.Settings = nil

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

local function kv(line, key)
    return line:match(key .. '="([^"]*)"')
end

local function findFrame(name)
    for _, f in ipairs(CAPTURE.frames) do
        if f._name == name then
            return f
        end
    end
    return nil
end

local function statusText()
    for _, fs in ipairs(CAPTURE.fontstrings) do
        if fs._text and has(fs._text, "bridge active") then
            return fs._text
        end
    end
    return nil
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

print("options panel tests (" .. OPTIONS .. ")")

local coreChunk, coreErr = loadfile(CORE)
assert(coreChunk, "cannot load core.lua: " .. tostring(coreErr))
coreChunk()

local optChunk, optErr = loadfile(OPTIONS)
assert(optChunk, "cannot load options.lua: " .. tostring(optErr))
optChunk()

-- ---------------------------------------------------------------- tests ---

do -- 1: panel registers with the classic Interface Options API
    local panel = Azeradio.OptionsPanel
    check("options panel is exposed", panel ~= nil)
    check("panel registers via InterfaceOptions_AddCategory", REGISTERED.classic == panel)
    check("panel is named Azeradio", panel and panel.name == "Azeradio")
end

local combatCheck = findFrame("AzeradioOptionsCombatCheck")
local logCheck = findFrame("AzeradioOptionsLogCheck")
local testButton = findFrame("AzeradioOptionsTestButton")

do -- 2: combat checkbox mirrors live state with a tooltip
    check("combat checkbox exists", combatCheck ~= nil)
    check("combat checkbox is labelled", combatCheck and combatCheck._label == "Combat announcements", combatCheck and tostring(combatCheck._label))
    check("combat defaults on", combatCheck and combatCheck:GetChecked() == true)
    check("combat checkbox has a tooltip", combatCheck and combatCheck.scripts.OnEnter ~= nil and combatCheck.scripts.OnLeave ~= nil)
    TOOLTIP.shown, TOOLTIP.text, TOOLTIP.lines = false, nil, {}
    combatCheck.scripts.OnEnter(combatCheck)
    check("tooltip names the toggle", TOOLTIP.shown and TOOLTIP.text == "Combat announcements", tostring(TOOLTIP.text))
    combatCheck.scripts.OnLeave(combatCheck)
    check("tooltip hides on leave", TOOLTIP.shown == false)
end

do -- 3: unchecking combat silences combat lines, rechecking restores them
    local n = #CAPTURE.bridge
    combatCheck:SetChecked(false)
    combatCheck.scripts.OnClick(combatCheck)
    check("clicking off updates state", Azeradio.IsCombatAnnounce() == false)
    check("clicking off plays the off sound", CAPTURE.sounds[#CAPTURE.sounds] == 857, tostring(CAPTURE.sounds[#CAPTURE.sounds]))
    check("status shows combat off", has(statusText() or "", "combat off"), statusText())
    MOCK.combat = true
    fire("PLAYER_REGEN_DISABLED")
    check("combat off suppresses combat lines", #CAPTURE.bridge == n)
    MOCK.combat = false
    combatCheck:SetChecked(true)
    combatCheck.scripts.OnClick(combatCheck)
    check("clicking on restores state", Azeradio.IsCombatAnnounce() == true)
    fire("PLAYER_REGEN_ENABLED")
    check("combat on restores combat lines", #CAPTURE.bridge == n + 1)
end

do -- 4: logging checkbox drives chat logging with a restart reminder
    check("logging checkbox exists", logCheck ~= nil)
    check("logging checkbox reflects game state", logCheck and logCheck:GetChecked() == true)
    local c = #CAPTURE.chat
    logCheck:SetChecked(false)
    logCheck.scripts.OnClick(logCheck)
    check("logging off disables LoggingChat", MOCK.chatlog == false)
    check("status shows chat log off", has(statusText() or "", "chat log off"), statusText())
    logCheck:SetChecked(true)
    logCheck.scripts.OnClick(logCheck)
    check("logging on enables LoggingChat", MOCK.chatlog == true)
    check("logging on reminds about a full restart", #CAPTURE.chat == c + 1 and has(CAPTURE.chat[#CAPTURE.chat], "not /reload"), CAPTURE.chat[#CAPTURE.chat])
end

do -- 5: test button emits a bridge line plus a visible confirmation
    check("test button exists", testButton ~= nil and testButton.scripts.OnClick ~= nil)
    local b, c = #CAPTURE.bridge, #CAPTURE.chat
    testButton.scripts.OnClick(testButton)
    check("test button emits a bridge line", #CAPTURE.bridge == b + 1)
    check("test line is tagged why=test", kv(CAPTURE.bridge[#CAPTURE.bridge], "why") == "test")
    check(
        "test button confirms in chat",
        #CAPTURE.chat == c + 1 and has(CAPTURE.chat[#CAPTURE.chat], "test event sent"),
        CAPTURE.chat[#CAPTURE.chat]
    )
end

do -- 6: slash commands and the panel stay in sync through OnShow
    slash("combat off")
    Azeradio.OptionsPanel.scripts.OnShow(Azeradio.OptionsPanel)
    check("slash combat off unchecks the box on next show", combatCheck:GetChecked() == false)
    slash("combat on")
    Azeradio.OptionsPanel.scripts.OnShow(Azeradio.OptionsPanel)
    check("slash combat on rechecks the box on next show", combatCheck:GetChecked() == true)
end

do -- 7: mid-drag hover never pops a tooltip
    MOCK.drag = true
    TOOLTIP.shown, TOOLTIP.text = false, nil
    combatCheck.scripts.OnEnter(combatCheck)
    check("tooltip stays hidden mid-drag", TOOLTIP.shown == false)
    MOCK.drag = false
end

do -- 8: modern Settings API registers the same panel without errors
    _G.InterfaceOptions_AddCategory = nil
    _G.Settings = {
        RegisterCanvasLayoutCategory = function(p, name)
            REGISTERED.canvasPanel, REGISTERED.canvasName = p, name
            return { id = "azeradio-cat" }
        end,
        RegisterAddOnCategory = function(cat)
            REGISTERED.addonCat = cat
        end,
    }
    local rechunk, reerr = loadfile(OPTIONS)
    check("options reloads under the Settings API", rechunk ~= nil, reerr)
    if rechunk then
        rechunk()
        check("canvas category carries the panel", REGISTERED.canvasPanel == Azeradio.OptionsPanel)
        check("canvas category is named Azeradio", REGISTERED.canvasName == "Azeradio")
        check("category registers as an addon category", REGISTERED.addonCat and REGISTERED.addonCat.id == "azeradio-cat")
    end
end

do -- 9: no options API at all still loads silently (unavailable, not broken)
    _G.Settings = nil
    local bare, bareErr = loadfile(OPTIONS)
    local ok = bare and pcall(bare)
    check("options loads with no registration API", ok == true, bareErr)
end

-- -------------------------------------------------------------- report ---

print(("%d passed, %d failed"):format(passed, failed))
if failed > 0 then
    os.exit(1)
end
