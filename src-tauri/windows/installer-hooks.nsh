; Tauri's default NSIS template deliberately pauses on the completed
; installation-progress page so users can inspect its diagnostic log.
; SCRCPY Studio uses the normal finish page for its launch and shortcut
; choices, so advance there automatically after a successful installation.
!macro NSIS_HOOK_POSTINSTALL
  SetAutoClose true
!macroend
