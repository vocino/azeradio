-- Azeradio options.lua
-- Loaded after core.lua (see Azeradio_Forever.toc). Forever only.
--
-- What it does: registers the in-game Interface Options panel ("Azeradio")
-- so the addon shows up where players expect and visibly works: two live
-- toggles (combat announcements, chat logging) plus a test-event button.
-- Every control applies immediately with a checkbox sound; nothing here
-- needs a reload. Slash commands in core.lua stay the source of truth for
-- state -- this panel only reads and writes through the Azeradio.* API.
--
-- Client coverage: the Forever client uses the classic Interface Options
-- API (InterfaceOptions_AddCategory). When the modern Settings API exists
-- instead, register the same panel there. Every Blizzard call is guarded
-- so a missing API leaves the panel unregistered, never erroring.

local PANEL_NAME = "Azeradio"

if not CreateFrame or not UIParent then
    return
end
if not Azeradio then
    return
end

-- Confirmation sound on every toggle, with a numeric fallback so a
-- Blizzard rename never silently kills the polish.
local function play(which)
    if not PlaySound then
        return
    end
    local key = (which == "off") and "IG_MAINMENU_OPTION_CHECKBOX_OFF" or "IG_MAINMENU_OPTION_CHECKBOX_ON"
    local id = (which == "off") and 857 or 856
    if SOUNDKIT and SOUNDKIT[key] then
        pcall(PlaySound, SOUNDKIT[key])
    else
        pcall(PlaySound, id)
    end
end

-- One-line tooltip per toggle; never fires mid-drag.
local function setTooltip(control, title, line)
    if not control or not control.SetScript then
        return
    end
    control:SetScript("OnEnter", function(self)
        if IsMouseButtonDown and IsMouseButtonDown() then
            return
        end
        if not GameTooltip then
            return
        end
        GameTooltip:SetOwner(self, "ANCHOR_RIGHT")
        GameTooltip:SetText(title)
        if line and GameTooltip.AddLine then
            GameTooltip:AddLine(line)
        end
        if GameTooltip.Show then
            GameTooltip:Show()
        end
    end)
    control:SetScript("OnLeave", function()
        if GameTooltip and GameTooltip.Hide then
            GameTooltip:Hide()
        end
    end)
end

local function makeLabel(parent, template, text, x, y)
    if not parent or not parent.CreateFontString then
        return nil
    end
    local fs = parent:CreateFontString(nil, "ARTWORK", template)
    if not fs then
        return nil
    end
    if fs.SetPoint then
        fs:SetPoint("TOPLEFT", parent, "TOPLEFT", x, y)
    end
    if fs.SetJustifyH then
        fs:SetJustifyH("LEFT")
    end
    if fs.SetText then
        fs:SetText(text)
    end
    return fs
end

local function setCheckLabel(cb, name, label)
    local fs = _G and _G[name .. "Text"] or nil
    if fs == nil and cb then
        fs = cb.Text
    end
    if fs and fs.SetText then
        fs:SetText(label)
        return
    end
    if cb and cb.CreateFontString then
        local own = cb:CreateFontString(nil, "ARTWORK", "GameFontNormal")
        if own then
            if own.SetPoint then
                own:SetPoint("LEFT", cb, "RIGHT", 4, 0)
            end
            if own.SetText then
                own:SetText(label)
            end
        end
    end
end

local function makeCheckbox(parent, name, label, tooltipLine, anchorTo, x, y)
    local cb = nil
    local ok, frame = pcall(CreateFrame, "CheckButton", name, parent, "UICheckButtonTemplate")
    if ok and frame then
        cb = frame
    else
        cb = CreateFrame("CheckButton", name, parent)
    end
    if not cb then
        return nil
    end
    if cb.SetPoint then
        cb:SetPoint("TOPLEFT", anchorTo, "BOTTOMLEFT", x, y)
    end
    setCheckLabel(cb, name, label)
    setTooltip(cb, label, tooltipLine)
    return cb
end

local panel = CreateFrame("Frame", "AzeradioOptionsPanel", UIParent)
if not panel then
    return
end
panel.name = PANEL_NAME

local title = makeLabel(panel, "GameFontNormalLarge", "Azeradio", 16, -16)
local desc = makeLabel(
    panel,
    "GameFontHighlight",
    "Zone-triggered Spotify soundtracks. The bridge writes invisible lines to the chat log; the tray app turns them into music.",
    16,
    -44
)
if desc and desc.SetWidth then
    desc:SetWidth(560)
end
local status = makeLabel(panel, "GameFontHighlight", "", 16, -92)

local combatCheck = makeCheckbox(
    panel,
    "AzeradioOptionsCombatCheck",
    "Combat announcements",
    "Announce combat start and end to the tray app.",
    status or panel,
    0,
    -16
)
local logCheck = makeCheckbox(
    panel,
    "AzeradioOptionsLogCheck",
    "Chat logging (WoWChatLog.txt)",
    "The tray app reads this file. Turning it on needs a full game restart, not /reload.",
    combatCheck or status or panel,
    0,
    -8
)
local logHint = makeLabel(panel, "GameFontDisableSmall", "Turning logging on needs a full game restart (not /reload).", 48, -8)
if logHint and combatCheck then
    -- Pin the hint under the logging checkbox once both exist.
    if logHint.ClearAllPoints and logHint.SetPoint then
        logHint:ClearAllPoints()
        logHint:SetPoint("TOPLEFT", logCheck, "BOTTOMLEFT", 32, -4)
    end
end

local testButton = nil
do
    local ok, btn = pcall(CreateFrame, "Button", "AzeradioOptionsTestButton", panel, "UIPanelButtonTemplate")
    if not (ok and btn) then
        btn = CreateFrame("Button", "AzeradioOptionsTestButton", panel)
    end
    testButton = btn
end
if testButton then
    if testButton.SetSize then
        testButton:SetSize(160, 24)
    end
    if testButton.SetPoint then
        if logHint then
            testButton:SetPoint("TOPLEFT", logHint, "BOTTOMLEFT", -32, -12)
        elseif logCheck then
            testButton:SetPoint("TOPLEFT", logCheck, "BOTTOMLEFT", 0, -28)
        end
    end
    if testButton.SetText then
        testButton:SetText("Send test event")
    end
    setTooltip(testButton, "Send test event", "Writes one invisible bridge line so the tray status line can prove it hears you.")
end
local usage = makeLabel(panel, "GameFontDisableSmall", "Slash: /azeradio combat on|off, /azeradio log on|off, /azeradio test", 16, -260)

local function refresh()
    if combatCheck and combatCheck.SetChecked and Azeradio.IsCombatAnnounce then
        combatCheck:SetChecked(Azeradio.IsCombatAnnounce())
    end
    if logCheck and logCheck.SetChecked and Azeradio.IsLogging then
        logCheck:SetChecked(Azeradio.IsLogging())
    end
    if status and status.SetText and Azeradio.StatusText then
        status:SetText(Azeradio.StatusText())
    end
end

if panel.SetScript then
    panel:SetScript("OnShow", refresh)
end

if combatCheck and combatCheck.SetScript then
    combatCheck:SetScript("OnClick", function(self)
        local on = false
        if self.GetChecked then
            on = self:GetChecked() and true or false
        end
        if Azeradio.SetCombatAnnounce then
            Azeradio.SetCombatAnnounce(on)
        end
        play(on and "on" or "off")
        refresh()
    end)
end

if logCheck and logCheck.SetScript then
    logCheck:SetScript("OnClick", function(self)
        local on = false
        if self.GetChecked then
            on = self:GetChecked() and true or false
        end
        if Azeradio.SetLogging then
            Azeradio.SetLogging(on)
        end
        play(on and "on" or "off")
        if on and DEFAULT_CHAT_FRAME and DEFAULT_CHAT_FRAME.AddMessage then
            DEFAULT_CHAT_FRAME:AddMessage("[AZERADIO] chat logging on -- quit and restart the game fully (not /reload)")
        end
        refresh()
    end)
end

if testButton and testButton.SetScript then
    testButton:SetScript("OnClick", function()
        if Azeradio.SendTest then
            Azeradio.SendTest()
        end
        play("on")
        if DEFAULT_CHAT_FRAME and DEFAULT_CHAT_FRAME.AddMessage then
            DEFAULT_CHAT_FRAME:AddMessage("[AZERADIO] test event sent (invisible) -- check the tray status line")
        end
        refresh()
    end)
end

refresh()

if Settings and Settings.RegisterCanvasLayoutCategory then
    local ok, category = pcall(Settings.RegisterCanvasLayoutCategory, panel, PANEL_NAME)
    if ok and category then
        if Settings.RegisterAddOnCategory then
            pcall(Settings.RegisterAddOnCategory, category)
        end
        panel.settingsCategory = category
    end
elseif Settings and Settings.RegisterAddOnCategory then
    local ok, category = pcall(Settings.RegisterAddOnCategory, panel, PANEL_NAME)
    if ok and category then
        panel.settingsCategory = category
    end
elseif InterfaceOptions_AddCategory then
    pcall(InterfaceOptions_AddCategory, panel)
end

Azeradio.OptionsPanel = panel
