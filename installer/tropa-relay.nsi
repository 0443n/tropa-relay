; tropa-relay NSIS installer
; User-level install — no UAC prompt required

!include "MUI2.nsh"

; --- General ---
Name "Tropa Relay"
OutFile "tropa-relay-${TAG}-windows-amd64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\tropa-relay"
RequestExecutionLevel user

; --- Interface ---
!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_ABORTWARNING

; --- Pages ---
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; --- Default section (required) ---
Section "Tropa Relay (required)" SecMain
    SectionIn RO

    SetOutPath "$INSTDIR"
    File "tropa-relay.exe"
    File "icon.ico"

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Start Menu shortcut
    CreateDirectory "$SMPROGRAMS\Tropa Relay"
    CreateShortCut "$SMPROGRAMS\Tropa Relay\Tropa Relay.lnk" "$INSTDIR\tropa-relay.exe" "" "$INSTDIR\icon.ico"
    CreateShortCut "$SMPROGRAMS\Tropa Relay\Uninstall.lnk" "$INSTDIR\uninstall.exe"

    ; Add/Remove Programs registry entry
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "DisplayName" "Tropa Relay"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "DisplayIcon" "$INSTDIR\icon.ico"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "Publisher" "tropa-relay"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "NoRepair" 1

    ; Estimate installed size (KB)
    SectionGetSize ${SecMain} $0
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay" "EstimatedSize" $0
SectionEnd

; --- Optional desktop shortcut ---
Section "Desktop shortcut" SecDesktop
    CreateShortCut "$DESKTOP\Tropa Relay.lnk" "$INSTDIR\tropa-relay.exe" "" "$INSTDIR\icon.ico"
SectionEnd

; --- Section descriptions ---
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} "Install Tropa Relay and create Start Menu shortcut."
    !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create a shortcut on the Desktop."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; --- Uninstaller ---
Section "Uninstall"
    ; Remove installed files
    Delete "$INSTDIR\tropa-relay.exe"
    Delete "$INSTDIR\icon.ico"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    ; Remove shortcuts
    Delete "$SMPROGRAMS\Tropa Relay\Tropa Relay.lnk"
    Delete "$SMPROGRAMS\Tropa Relay\Uninstall.lnk"
    RMDir "$SMPROGRAMS\Tropa Relay"
    Delete "$DESKTOP\Tropa Relay.lnk"

    ; Remove registry entry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\tropa-relay"

    ; NOTE: We intentionally do NOT remove:
    ;   %APPDATA%\tropa-relay\  (user config)
    ;   HKCU\Software\Microsoft\Windows\CurrentVersion\Run\tropa-relay  (autostart)
SectionEnd
