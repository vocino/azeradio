-- Azeradio core.lua
-- Shared by Azeradio.toc (Midnight) and Azeradio_Forever.toc (Forever).
-- The client loads whichever .toc has the highest Interface number that does
-- not exceed its own, so one folder serves both games.
--
-- What it does: on every zone change (and, optionally, combat start/end) it
-- prints one machine-readable line into chat:
--   [AZERADIO] zone="..." subzone="..." instance="..." instanceID="..." combat="0|1" why="..."
-- The Azeradio tray app tails Logs/WoWChatLog.txt, parses those lines, and
-- switches Spotify playlists. Turn on chat logging in game for this to work.

local PREFIX = "[AZERADIO]"

local frame = CreateFrame("Frame")
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("ZONE_CHANGED_NEW_AREA")
frame:RegisterEvent("ZONE_CHANGED")
frame:RegisterEvent("ZONE_CHANGED_INDOORS")
frame:RegisterEvent("PLAYER_REGEN_DISABLED")
frame:RegisterEvent("PLAYER_REGEN_ENABLED")

local combatAnnounce = true

local function esc(s)
    return (tostring(s or ""):gsub('"', "'"))
end

local function announce(why)
    local zone = GetZoneText() or ""
    local subzone = GetMinimapZoneText() or ""
    local _, instanceType, _, _, _, _, _, instanceID = GetInstanceInfo()
    local combat = InCombatLockdown() and "1" or "0"
    DEFAULT_CHAT_FRAME:AddMessage(
        ('%s zone="%s" subzone="%s" instance="%s" instanceID="%s" combat="%s" why="%s"'):format(
            PREFIX,
            esc(zone),
            esc(subzone),
            esc(instanceType or "none"),
            esc(instanceID or 0),
            combat,
            why
        )
    )
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
        combatAnnounce = true
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " combat announcements on")
    elseif msg == "combat off" then
        combatAnnounce = false
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " combat announcements off")
    elseif msg == "test" then
        announce("test")
    else
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " bridge active (combat " .. (combatAnnounce and "on" or "off") .. ")")
        DEFAULT_CHAT_FRAME:AddMessage(PREFIX .. " usage: /azeradio combat on|off, /azeradio test")
    end
end
