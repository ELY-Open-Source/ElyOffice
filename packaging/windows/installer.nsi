; Per-user installer; pass each define the checks below name.
Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef APP_NAME
  !error "pass -DAPP_NAME"
!endif
!ifndef APP_SLUG
  !error "pass -DAPP_SLUG"
!endif
!ifndef VERSION
  !error "pass -DVERSION"
!endif
!ifndef STAGE
  !error "pass -DSTAGE, a folder from packaging/stage.sh"
!endif
!ifndef ICON
  !error "pass -DICON"
!endif
!ifndef TYPES
  !error "pass -DTYPES, from packaging/windows/types.sh"
!endif
!include "${TYPES}"
!ifndef LIST
  !error "pass -DLIST, from packaging/windows/files.sh"
!endif
!ifndef OUTFILE
  !define OUTFILE "${APP_SLUG}-${VERSION}-setup.exe"
!endif
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ely Office ${APP_NAME}"

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
Name "${APP_NAME}"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\Ely Office\${APP_NAME}"
InstallDirRegKey HKCU "Software\Ely Office\${APP_NAME}" "InstallDir"
SetCompressor /SOLID lzma
AllowSkipFiles off

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

LangString DeleteData ${LANG_SIMPCHINESE} "同时删除 ${APP_NAME} 的缓存、账号与本地历史吗？你的文档不受影响。"
LangString DeleteData ${LANG_ENGLISH} "Also delete the cache, accounts and local history of ${APP_NAME}? Your documents stay."

LangString Busy ${LANG_SIMPCHINESE} "${APP_NAME} 的文件正被占用，未作任何改动。请关闭 ${APP_NAME} 后重试："
LangString Busy ${LANG_ENGLISH} "A file of ${APP_NAME} is in use; nothing was changed. Close ${APP_NAME} and try again:"
LangString Stuck ${LANG_SIMPCHINESE} "旧版的文件放回原处失败，留在："
LangString Stuck ${LANG_ENGLISH} "Could not put the old files back; they are in:"
LangString Left ${LANG_SIMPCHINESE} "新版已装好，旧版的文件删不掉，留在："
LangString Left ${LANG_ENGLISH} "The new version is installed; the old files could not be deleted from:"
LangString NoList ${LANG_SIMPCHINESE} "找不到安装清单，未删除任何文件："
LangString NoList ${LANG_ENGLISH} "No install list found; nothing was deleted:"

LangString Taken ${LANG_SIMPCHINESE} "安装位置已有不属于 ${APP_NAME} 的文件，未作任何改动："
LangString Taken ${LANG_ENGLISH} "A file ${APP_NAME} did not install is in the way; nothing was changed:"
LangString Types ${LANG_SIMPCHINESE} "${APP_NAME} 已装好，但文件类型登记失败，“打开方式”里没有它："
LangString Types ${LANG_ENGLISH} "${APP_NAME} is installed, but its file types could not be registered; Open With will not list it:"
LangString Untyped ${LANG_SIMPCHINESE} "${APP_NAME} 已卸下，但文件类型的登记删不掉，仍留在："
LangString Untyped ${LANG_ENGLISH} "${APP_NAME} is uninstalled, but its file type registration could not be removed from:"

; Lists a legacy folder's files and folders to $R2.
Function ElyWalk
  Push "|"
  Push $R1
walk:
  Pop $R1
  StrCmp $R1 "|" done
  FileWrite $R2 "$R1\$\r$\n"
  FindFirst $R3 $R4 "$INSTDIR\$R1\*"
entry:
  StrCmp $R4 "" closed
  StrCmp $R4 "." skip
  StrCmp $R4 ".." skip
  IfFileExists "$INSTDIR\$R1\$R4\*.*" 0 file
  Push "$R1\$R4"
  Goto skip
file:
  FileWrite $R2 "$R1\$R4$\r$\n"
skip:
  FindNext $R3 $R4
  Goto entry
closed:
  FindClose $R3
  Goto walk
done:
FunctionEnd

!macro ElyLegacyDir NAME
  IfFileExists "$INSTDIR\${NAME}\*.*" 0 +3
    StrCpy $R1 "${NAME}"
    Call ElyWalk
!macroend

!macro ElyLegacy ID PATTERN
  FindFirst $R3 $R4 "$INSTDIR\${PATTERN}"
  ely_legacy_${ID}:
    StrCmp $R4 "" ely_legacy_end_${ID}
    FileWrite $R2 "$R4$\r$\n"
    FindNext $R3 $R4
    Goto ely_legacy_${ID}
  ely_legacy_end_${ID}:
  FindClose $R3
!macroend

; Sets $R0 to the list an earlier install owns.
Function ElyOwned
  StrCpy $R0 ""
  StrCpy $R7 ""
  ; A copy, as this install rewrites the list.
  IfFileExists "$INSTDIR\.ely-installed" 0 unlisted
  StrCpy $R0 "$PLUGINSDIR\owned.txt"
  StrCpy $R7 "listed"
  CopyFiles /SILENT "$INSTDIR\.ely-installed" "$R0"
  Goto done
unlisted:
  ; Older releases: only the folder our key names.
  ReadRegStr $R1 HKCU "Software\Ely Office\${APP_NAME}" "InstallDir"
  StrCmp $R1 "$INSTDIR" 0 done
  IfFileExists "$INSTDIR\${APP_SLUG}.exe" 0 done
  StrCpy $R0 "$PLUGINSDIR\legacy.txt"
  FileOpen $R2 "$R0" w
  !insertmacro ElyLegacyDir libreoffice
  !insertmacro ElyLegacyDir tessdata
  !insertmacro ElyLegacyDir git
  !insertmacro ElyLegacyDir content
  !insertmacro ElyLegacyDir locales
  !insertmacro ElyLegacyDir licenses
  FileWrite $R2 "${APP_SLUG}.ico$\r$\n"
  !insertmacro ElyLegacy exe "*.exe"
  !insertmacro ElyLegacy dll "*.dll"
  !insertmacro ElyLegacy pak "*.pak"
  !insertmacro ElyLegacy bin "*.bin"
  !insertmacro ElyLegacy dat "*.dat"
  !insertmacro ElyLegacy json "*.json"
  FileClose $R2
done:
FunctionEnd

!macro ElyAside UN
; Next line of $R2: path $R3, kind $R6.
Function ${UN}ElyNext
next:
  ClearErrors
  FileRead $R2 $R3
  IfErrors 0 trim
    StrCpy $R6 ""
    Return
trim:
  StrCpy $R6 "$R3" 1 -1
  StrCmp $R6 "$\r" cut
  StrCmp $R6 "$\n" cut
  StrCmp $R3 "" next
  StrCmp $R6 "\" 0 +4
    StrCpy $R3 "$R3" -1
    StrCpy $R6 "d"
    Return
  StrCpy $R6 "f"
  Return
cut:
  StrCpy $R3 "$R3" -1
  Goto trim
FunctionEnd

; Removes the empty folders $R0 lists under $R1.
Function ${UN}ElyPrune
pass:
  StrCpy $8 ""
  FileOpen $R2 "$R0" r
next:
  Call ${UN}ElyNext
  StrCmp $R6 "" passed
  StrCmp $R6 "f" next
  IfFileExists "$R1\$R3\*.*" 0 next
  ClearErrors
  RMDir "$R1\$R3"
  IfErrors next
  StrCpy $8 "again"
  Goto next
passed:
  FileClose $R2
  StrCmp $8 "again" pass
FunctionEnd

; Moves $R0's files to numbered .ely-old entries; $R5: blocker.
Function ${UN}ElyAside
  StrCpy $R5 ""
  StrCpy $R9 "$INSTDIR\.ely-old"
  RMDir /r "$R9"
  IfFileExists "$R9" 0 +3
    StrCpy $R5 "$R9"
    Return
  ClearErrors
  FileOpen $R2 "$R0" r
  IfErrors 0 check
    StrCpy $R5 "$R0"
    Return
check:
  ; Every file first: running or locked ones refuse writing.
  Call ${UN}ElyNext
  StrCmp $R6 "" checked
  StrCmp $R6 "d" check
  IfFileExists "$INSTDIR\$R3\*.*" check
  IfFileExists "$INSTDIR\$R3" 0 check
  ClearErrors
  FileOpen $R4 "$INSTDIR\$R3" a
  IfErrors refused
  FileClose $R4
  Goto check
checked:
  CreateDirectory "$R9"
  FileSeek $R2 0 SET
  ; Numbered names keep backup paths short.
  StrCpy $9 0
move:
  Call ${UN}ElyNext
  StrCmp $R6 "" moved
  StrCmp $R6 "d" move
  IntOp $9 $9 + 1
  IfFileExists "$INSTDIR\$R3\*.*" move
  IfFileExists "$INSTDIR\$R3" 0 move
  ClearErrors
  Rename "$INSTDIR\$R3" "$R9\$9"
  IfErrors 0 move
  StrCpy $R5 "$INSTDIR\$R3"
  FileClose $R2
  Call ${UN}ElyBack
  Return
moved:
  FileClose $R2
  Return
refused:
  StrCpy $R5 "$INSTDIR\$R3"
  FileClose $R2
FunctionEnd

; Puts $R0's files back from .ely-old.
Function ${UN}ElyBack
  FileOpen $R2 "$R0" r
  StrCpy $9 0
back:
  Call ${UN}ElyNext
  StrCmp $R6 "" done
  StrCmp $R6 "d" back
  IntOp $9 $9 + 1
  IfFileExists "$R9\$9" 0 back
  Rename "$R9\$9" "$INSTDIR\$R3"
  Goto back
done:
  FileClose $R2
  RMDir "$R9"
  IfFileExists "$R9" 0 +2
    MessageBox MB_ICONSTOP "$(Stuck)$\r$\n$R9" /SD IDOK
FunctionEnd
!macroend
!insertmacro ElyAside ""
!insertmacro ElyAside "un."

; Sets $R5 to a path in the way.
Function ElyClear
  StrCpy $R5 ""
  FileOpen $R2 "$PLUGINSDIR\new.txt" r
  FileOpen $7 "$PLUGINSDIR\made.txt" w
next:
  Call ElyNext
  StrCmp $R6 "" done
  StrCmp $R6 "d" folder
  IfFileExists "$INSTDIR\$R3" taken next
folder:
  IfFileExists "$INSTDIR\$R3\*.*" next
  IfFileExists "$INSTDIR\$R3" taken
  FileWrite $7 "$R3\$\r$\n"
  Goto next
taken:
  StrCpy $R5 "$INSTDIR\$R3"
done:
  FileClose $7
  FileClose $R2
FunctionEnd

; Removes what this install wrote, then restores the old.
Function .onInstFailed
  StrCmp $R8 "copying" 0 done
  FileOpen $R2 "$PLUGINSDIR\new.txt" r
drop:
  Call ElyNext
  StrCmp $R6 "" dropped
  StrCmp $R6 "d" drop
  Delete "$INSTDIR\$R3"
  Goto drop
dropped:
  FileClose $R2
  Delete "$INSTDIR\.ely-installed"
  Push $R0
  StrCpy $R0 "$PLUGINSDIR\made.txt"
  StrCpy $R1 "$INSTDIR"
  Call ElyPrune
  Pop $R0
  StrCmp $R0 "" done
  Call ElyBack
  StrCmp $R7 "listed" 0 done
  CopyFiles /SILENT "$R0" "$INSTDIR\.ely-installed"
done:
FunctionEnd

Section "Install"
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "/oname=new.txt" "${LIST}"
  ; An earlier install moves aside whole, or nothing changes.
  Call ElyOwned
  StrCmp $R0 "" clear
  Call ElyAside
  StrCmp $R5 "" clear
  MessageBox MB_ICONSTOP "$(Busy)$\r$\n$R5" /SD IDOK
  SetErrorLevel 2
  Quit
clear:
  ; Never write over a file this app did not install.
  Call ElyClear
  StrCmp $R5 "" copy
  StrCmp $R0 "" +2
    Call ElyBack
  MessageBox MB_ICONSTOP "$(Taken)$\r$\n$R5" /SD IDOK
  SetErrorLevel 2
  Quit
copy:
  StrCpy $R8 "copying"
  ; A file that cannot be written fails the install.
  ClearErrors
  SetOutPath "$INSTDIR"
  File /r "${STAGE}\*.*"
  File "/oname=${APP_SLUG}.ico" "${ICON}"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CopyFiles /SILENT "$PLUGINSDIR\new.txt" "$INSTDIR\.ely-installed"
  IfErrors 0 +2
    Abort
  StrCpy $R8 ""
  ; Old folders the new release leaves empty.
  StrCmp $R0 "" +3
    StrCpy $R1 "$INSTDIR"
    Call ElyPrune
  RMDir /r "$INSTDIR\.ely-old"
  IfFileExists "$INSTDIR\.ely-old" 0 +2
    MessageBox MB_ICONEXCLAMATION "$(Left)$\r$\n$INSTDIR\.ely-old" /SD IDOK
  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${APP_SLUG}.exe" "" "$INSTDIR\${APP_SLUG}.ico"
  WriteRegStr HKCU "Software\Ely Office\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "ZacharyZhang"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://zacharyzhang.com"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${APP_SLUG}.ico"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  ; The types macro sets errors, $2 naming the key.
  ClearErrors
  !insertmacro ElyTypesInstall
  IfErrors 0 typed
    MessageBox MB_ICONSTOP "$(Types)$\r$\n$2" /SD IDOK
    Abort
typed:
SectionEnd

Section "Uninstall"
  StrCpy $R5 "$INSTDIR\.ely-installed"
  IfFileExists "$R5" listed
  MessageBox MB_ICONSTOP "$(NoList)$\r$\n$INSTDIR" /SD IDOK
  SetErrorLevel 2
  Quit
listed:
  ; The list is checked too, before anything moves.
  ClearErrors
  FileOpen $R4 "$R5" a
  IfErrors busy
  FileClose $R4
  InitPluginsDir
  StrCpy $R0 "$PLUGINSDIR\owned.txt"
  CopyFiles /SILENT "$R5" "$R0"
  Call un.ElyAside
  StrCmp $R5 "" gone busy
gone:
  ClearErrors
  Delete "$INSTDIR\.ely-installed"
  IfErrors 0 deleted
  Call un.ElyBack
  StrCpy $R5 "$INSTDIR\.ely-installed"
busy:
  MessageBox MB_ICONSTOP "$(Busy)$\r$\n$R5" /SD IDOK
  SetErrorLevel 2
  Quit
deleted:
  RMDir /r "$INSTDIR\.ely-old"
  StrCpy $R1 "$INSTDIR"
  Call un.ElyPrune
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  DeleteRegKey HKCU "Software\Ely Office\${APP_NAME}"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  ; The types macro sets errors, $2 naming the key.
  ClearErrors
  !insertmacro ElyTypesUninstall
  IfErrors 0 untyped
    MessageBox MB_ICONSTOP "$(Untyped)$\r$\n$2" /SD IDOK
    SetErrorLevel 2
untyped:
  MessageBox MB_YESNO|MB_DEFBUTTON2 "$(DeleteData)" /SD IDNO IDNO done
  RMDir /r "$LOCALAPPDATA\ElyOffice\${APP_SLUG}"
done:
SectionEnd
