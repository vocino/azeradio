-- Azeradio core.lua
-- Loaded by Azeradio_Forever.toc. Forever only (Interface 16001).
--
-- What it does: on every zone change (and, optionally, combat start/end) it
-- writes one machine-readable line to the chat log file (never shown on screen):
--   [AZERADIO] zone="..." subzone="..." instance="..." instanceName="..." instanceID="..." combat="0|1" why="..."
-- The Azeradio tray app tails Logs/WoWChatLog.txt, parses those lines, and
-- switches Spotify playlists. Turn on chat logging in game for this to work.

local PREFIX = "[AZERADIO]"

-- Hidden log window: chat logging (Logs/WoWChatLog.txt) only captures real
-- chat frames (FCF windows). A custom ScrollingMessageFrame is never logged,
-- so the bridge lines would silently reach nobody. We open one dedicated FCF
-- window, strip its groups/channels, park it off-screen at 1px, and hide its
-- tab: the player never sees a thing, but every AddMessage is logged exactly
-- like any ordinary chat window.

local WINDOW_NAME = "AzeradioLog"

local function setupLogFrame(f)
    if FCF_SetWindowName then
        FCF_SetWindowName(f, WINDOW_NAME)
    end
    if FCF_UnDockFrame then
        FCF_UnDockFrame(f)
    end
    if FCF_SetLocked then
        FCF_SetLocked(f, 1)
    end
    if ChatFrame_RemoveAllMessageGroups then
        ChatFrame_RemoveAllMessageGroups(f)
    end
    if ChatFrame_RemoveAllChannels then
        ChatFrame_RemoveAllChannels(f)
    end
    f:SetClampedToScreen(false)
    f:ClearAllPoints()
    f:SetPoint("TOPLEFT", UIParent, "TOPLEFT", -10, 10)
    f:SetSize(1, 1)
    local tab = _G[f:GetName() .. "Tab"]
    if tab then
        tab:Hide()
    end
    if FCF_SelectDockFrame then
        FCF_SelectDockFrame(ChatFrame1)
    end
    return f
end

local function acquireLogFrame()
    for i = 1, NUM_CHAT_WINDOWS do
        local f = _G["ChatFrame" .. i]
        if f and f.name == WINDOW_NAME then
            return setupLogFrame(f)
        end
    end
    if FCF_OpenNewWindow then
        FCF_OpenNewWindow(WINDOW_NAME)
        for i = 1, NUM_CHAT_WINDOWS do
            local f = _G["ChatFrame" .. i]
            if f and f.name == WINDOW_NAME then
                return setupLogFrame(f)
            end
        end
        if FCF_GetCurrentChatFrame then
            local f = FCF_GetCurrentChatFrame()
            if f then
                return setupLogFrame(f)
            end
        end
    end
    return nil
end

local logFrame = acquireLogFrame()
if not logFrame then
    -- Last-resort fallback: visible nowhere, but also never chat-logged.
    logFrame = CreateFrame("ScrollingMessageFrame", "AzeradioLogFrame", UIParent)
    logFrame:SetSize(1, 1)
    logFrame:SetPoint("TOPLEFT", UIParent, "TOPLEFT", -10, 10)
    logFrame:SetMaxLines(50)
    logFrame:SetFontObject(ChatFontNormal)
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("ZONE_CHANGED_NEW_AREA")
frame:RegisterEvent("ZONE_CHANGED")
frame:RegisterEvent("ZONE_CHANGED_INDOORS")
frame:RegisterEvent("PLAYER_REGEN_DISABLED")
frame:RegisterEvent("PLAYER_REGEN_ENABLED")

local combatAnnounce = true

-- Chat logging is a real API (LoggingChat), not a CVar: the old
-- SetCVar("chatLog", ...) silently did nothing, so the log file was never
-- created. CVar read kept as a fallback for clients lacking the API.
local function isLogging()
    if LoggingChat then
        return LoggingChat()
    end
    return GetCVar("chatLog") == "1"
end

local function logState()
    return isLogging() and "on" or "off"
end

local function setLogging(on)
    if LoggingChat then
        LoggingChat(on and true or false)
    else
        SetCVar("chatLog", on and "1" or "0")
    end
end

local function esc(s)
    return (tostring(s or ""):gsub('"', "'"))
end

local function announce(why)
    local zone = GetZoneText() or ""
    local subzone = GetMinimapZoneText() or ""
    local instanceName, instanceType, _, _, _, _, _, instanceID = GetInstanceInfo()
    local combat = InCombatLockdown() and "1" or "0"
    logFrame:AddMessage(
        ('%s zone="%s" subzone="%s" instance="%s" instanceName="%s" instanceID="%s" combat="%s" why="%s"'):format(
            PREFIX,
            esc(zone),
            esc(subzone),
            esc(instanceType or "none"),
            esc(instanceName or ""),
            esc(instanceID or 0),
            combat,
            why
        )
    )
end

-- Shared state for the Interface Options panel (options.lua) and slash
-- commands. Single source of truth: both read and write through here.
Azeradio = Azeradio or {}
function Azeradio.IsCombatAnnounce()
    return combatAnnounce
end
function Azeradio.SetCombatAnnounce(on)
    combatAnnounce = on and true or false
end
function Azeradio.IsLogging()
    return isLogging()
end
function Azeradio.SetLogging(on)
    setLogging(on)
end
function Azeradio.SendTest()
    announce("test")
end
function Azeradio.StatusText()
    return PREFIX .. " bridge active (combat " .. (combatAnnounce and "on" or "off") .. ", chat log " .. logState() .. ")"
end

frame:SetScript("OnEvent", function(_, event)
    if event == "PLAYER_REGEN_DISABLED" or event == "PLAYER_REGEN_ENABLED" then
        if combatAnnounce then
            announce(event == "PLAYER_REGEN_DISABLED" and "combat_start" or "combat_end")
        end
    else
        announce("zone")
    end
end)

SLASH_AZERADIO1 = "/azeradio"
SlashCmdList["AZERADIO"] = function(msg)
    msg = strtrim(msg:lower())
    if msg == "combat on" then
        Azeradio.SetCombatAnnounce(true)
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " combat announcements on")
    elseif msg == "combat off" then
        Azeradio.SetCombatAnnounce(false)
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " combat announcements off")
    elseif msg == "log on" then
        setLogging(true)
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " chat logging " .. logState() .. " — quit and restart the game fully (not /reload)")
    elseif msg == "log off" then
        setLogging(false)
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " chat logging " .. logState())
    elseif msg == "log" then
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " chat logging is " .. logState())
    elseif msg == "test" then
        announce("test")
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " test event sent (invisible) — check the tray status line")
    else
        DEFAULT_CHAT_FRAME:AddMessage(Azeradio.StatusText())
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " usage: /azeradio combat on|off, /azeradio log on|off, /azeradio test")
    end
end
