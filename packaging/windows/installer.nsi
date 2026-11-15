; One per-user setup for the ticked apps.
Unicode true
ManifestDPIAware true
ManifestDPIAwareness PerMonitorV2
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "Sections.nsh"

!ifndef VERSION
  !error "pass -DVERSION"
!endif
!ifndef APPS
  !error "pass -DAPPS, the ElyAll macro"
!endif
!ifndef STAGES
  !error "pass -DSTAGES: the kernel, each app's stage, list, types"
!endif
!ifndef OUTFILE
  !error "pass -DOUTFILE"
!endif
!include "${APPS}"
!define UNINSTALL "Software\Microsoft\Windows\CurrentVersion\Uninstall"
!define RUN "Software\Microsoft\Windows\CurrentVersion\Run"

Name "Ely Office"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\Ely Office"
InstallDirRegKey HKCU "Software\Ely Office" "Root"
SetCompressor /SOLID lzma
AllowSkipFiles off

Var Root
Var AppName
Var AppSlug
Var Index
Var Data
Var Suite

!define MUI_DIRECTORYPAGE_TEXT_TOP "$(Where)"
!insertmacro MUI_PAGE_COMPONENTS
!define MUI_PAGE_CUSTOMFUNCTION_PRE ElyWhere
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_COMPONENTS
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

LangString Where ${LANG_SIMPCHINESE} "新装的应用各放在此文件夹下自己的子文件夹里；已装的应用在原处升级。"
LangString Where ${LANG_ENGLISH} "New apps each go in their own folder inside this one; installed apps update where they are."
LangString DeleteData ${LANG_SIMPCHINESE} "同时删除卸下的应用的缓存、账号与本地历史吗？你的文档不受影响。"
LangString DeleteData ${LANG_ENGLISH} "Also delete the cache, accounts and local history of the apps you remove? Your documents stay."
LangString Busy ${LANG_SIMPCHINESE} "$AppName 的文件正被占用，未作任何改动。请关闭 $AppName 后重试："
LangString Busy ${LANG_ENGLISH} "A file of $AppName is in use; nothing was changed. Close $AppName and try again:"
LangString Stuck ${LANG_SIMPCHINESE} "旧版的文件放回原处失败，留在："
LangString Stuck ${LANG_ENGLISH} "Could not put the old files back; they are in:"
LangString Left ${LANG_SIMPCHINESE} "新版已装好，旧版的文件删不掉，留在："
LangString Left ${LANG_ENGLISH} "The new version is installed; the old files could not be deleted from:"
LangString NoList ${LANG_SIMPCHINESE} "找不到 $AppName 的安装清单，未删除任何文件："
LangString NoList ${LANG_ENGLISH} "No install list for $AppName found; nothing was deleted:"
LangString Taken ${LANG_SIMPCHINESE} "安装位置已有不属于 $AppName 的文件，未作任何改动："
LangString Taken ${LANG_ENGLISH} "A file $AppName did not install is in the way; nothing was changed:"
LangString Types ${LANG_SIMPCHINESE} "$AppName 已装好，但文件类型登记失败，“打开方式”里没有它："
LangString Types ${LANG_ENGLISH} "$AppName is installed, but its file types could not be registered; Open With will not list it:"
LangString Untyped ${LANG_SIMPCHINESE} "$AppName 已卸下，但文件类型的登记删不掉，仍留在："
LangString Entry ${LANG_SIMPCHINESE} "Ely Office 的卸载程序或其登记写不进，安装已停止："
LangString Entry ${LANG_ENGLISH} "The Ely Office uninstaller or its entry could not be written; the install stopped:"
LangString Keyed ${LANG_SIMPCHINESE} "$AppName 的文件已装好，但快捷方式或登记写不进，安装已停止："
LangString Keyed ${LANG_ENGLISH} "The files of $AppName are installed, but its shortcut or entry could not be written; the install stopped:"
LangString Unread ${LANG_SIMPCHINESE} "$AppName 的登记读不出，已停止："
LangString Unread ${LANG_ENGLISH} "The entry of $AppName cannot be read; stopped:"
LangString Unlinked ${LANG_SIMPCHINESE} "$AppName 的文件已装好，但连不上共用的内核，安装已停止："
LangString Unlinked ${LANG_ENGLISH} "The files of $AppName are installed, but it could not be linked to the shared kernel; the install stopped:"
LangString Alone ${LANG_SIMPCHINESE} "等不到另一个 Ely Office 安装程序结束，未作任何改动："
LangString Alone ${LANG_ENGLISH} "Could not wait for another Ely Office setup to finish; nothing was changed:"
LangString Unkeyed ${LANG_SIMPCHINESE} "$AppName 已卸下，但其登记或快捷方式删不掉，仍留在："
LangString Unkeyed ${LANG_ENGLISH} "$AppName is uninstalled, but its entry or shortcut could not be removed from:"
LangString Rooted ${LANG_SIMPCHINESE} "Ely Office 已装在别处，只能在原处更新："
LangString Rooted ${LANG_ENGLISH} "Ely Office is installed elsewhere; it updates only there:"
LangString Unstopped ${LANG_SIMPCHINESE} "Ely Office 的空闲引擎停不下，未作任何改动："
LangString Unstopped ${LANG_ENGLISH} "The idle engines of Ely Office would not stop; nothing was changed:"
LangString Unwarmed ${LANG_SIMPCHINESE} "Ely Office 已装好，但空闲引擎没起来，打开文档会慢一些："
LangString Unwarmed ${LANG_ENGLISH} "Ely Office is installed, but its idle engines did not start; documents open slower:"
LangString Unrun ${LANG_SIMPCHINESE} "Ely Office 已装好，但登录后自启的登记写不进或删不掉："
LangString Unrun ${LANG_ENGLISH} "Ely Office is installed, but its start at login could not be written or removed:"
LangString Untyped ${LANG_ENGLISH} "$AppName is uninstalled, but its file type registration could not be removed from:"

!include "${__FILEDIR__}\transact.nsh"

; $1: app NAME's folder, or ""; unreadable quits.
!macro ElyKey NAME
  ClearErrors
  ReadRegStr $1 HKCU "Software\Ely Office\${NAME}" "InstallDir"
  ${If} ${Errors}
    System::Call 'advapi32::RegGetValueW(p 0x80000001, w "Software\Ely Office\${NAME}", w "InstallDir", i 2, p 0, p 0, p 0) i .r2'
    ${If} $2 != 2
      StrCpy $AppName "${NAME}"
      MessageBox MB_ICONSTOP "$(Unread)$\r$\nHKCU\Software\Ely Office\${NAME} ($2)" /SD IDOK
      SetErrorLevel 2
      Quit
    ${EndIf}
    ClearErrors
  ${EndIf}
!macroend

; Sets $INSTDIR to app NAME's folder.
!macro ElyDir NAME
  !insertmacro ElyKey ${NAME}
  StrCpy $INSTDIR $1
  ${If} $INSTDIR == ""
    StrCpy $INSTDIR "$Root\${NAME}"
  ${EndIf}
!macroend

; Deletes HKCU KEY; $0 is 0 or the error.
!macro ElyUnkey KEY
  System::Call 'shlwapi::SHDeleteKeyW(p 0x80000001, w "${KEY}") i .r0'
  ${If} $0 == 2
    StrCpy $0 0
  ${EndIf}
!macroend

; Removes app NAME's shortcut, keys and types.
!macro ElyForget NAME
  Delete "$SMPROGRAMS\${NAME}.lnk"
  ${If} ${FileExists} "$SMPROGRAMS\${NAME}.lnk"
    MessageBox MB_ICONSTOP "$(Unkeyed)$\r$\n$SMPROGRAMS\${NAME}.lnk" /SD IDOK
    SetErrorLevel 2
  ${EndIf}
  !insertmacro ElyUnkey "Software\Ely Office\${NAME}"
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "$(Unkeyed)$\r$\nHKCU\Software\Ely Office\${NAME} ($0)" /SD IDOK
    SetErrorLevel 2
  ${EndIf}
  !insertmacro ElyUnkey "${UNINSTALL}\Ely Office ${NAME}"
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "$(Unkeyed)$\r$\nHKCU\${UNINSTALL}\Ely Office ${NAME} ($0)" /SD IDOK
    SetErrorLevel 2
  ${EndIf}
  ClearErrors
  !insertmacro ElyTypesUninstall${NAME}
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "$(Untyped)$\r$\n$2" /SD IDOK
    SetErrorLevel 2
  ${EndIf}
!macroend

!include "${__FILEDIR__}\kernel.nsh"

!macro ElyApp NAME SLUG SHARES
!include "${STAGES}\${SLUG}-types.nsh"
Section "${NAME}"
  StrCpy $AppName "${NAME}"
  StrCpy $AppSlug "${SLUG}"
  !insertmacro ElyDir ${NAME}
  SetOutPath "$PLUGINSDIR"
  File "/oname=new.txt" "${STAGES}\${SLUG}-files.txt"
  Call ElyPrepare
  StrCpy $R8 "copying"
  ; A file that cannot be written fails the install.
  ClearErrors
  SetOutPath "$INSTDIR"
  File /r "${STAGES}\${SLUG}\*.*"
  CopyFiles /SILENT "$PLUGINSDIR\new.txt" "$INSTDIR\.ely-installed"
  ${If} ${Errors}
    Abort
  ${EndIf}
  Call ElyEmptied
  ; Linked before the commit: a failure restores the old.
  !if ${SHARES} == 1
    !insertmacro ElyLink
  !endif
  StrCpy $R8 ""
  Call ElyPlaced
  ClearErrors
  CreateShortcut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\${SLUG}.exe" "" "$INSTDIR\${SLUG}.ico"
  WriteRegStr HKCU "Software\Ely Office\${NAME}" "InstallDir" "$INSTDIR"
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "$(Keyed)$\r$\nHKCU\Software\Ely Office\${NAME}" /SD IDOK
    Abort
  ${EndIf}
  !insertmacro ElyUnkey "${UNINSTALL}\Ely Office ${NAME}"
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "$(Keyed)$\r$\nHKCU\${UNINSTALL}\Ely Office ${NAME} ($0)" /SD IDOK
    Abort
  ${EndIf}
  !insertmacro ElyTypesInstall${NAME}
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "$(Types)$\r$\n$2" /SD IDOK
    Abort
  ${EndIf}
SectionEnd
Section "un.${NAME}"
  !insertmacro ElyKey ${NAME}
  ${If} $1 != ""
    StrCpy $AppName "${NAME}"
    StrCpy $INSTDIR $1
    Call un.ElyRemove
    !insertmacro ElyUnlink ${SHARES}
    !insertmacro ElyForget ${NAME}
    ${If} $Data == ""
      StrCpy $Data "no"
      MessageBox MB_YESNO|MB_DEFBUTTON2 "$(DeleteData)" /SD IDNO IDNO +2
        StrCpy $Data "yes"
    ${EndIf}
    ${If} $Data == "yes"
      RMDir /r "$LOCALAPPDATA\ElyOffice\${SLUG}"
    ${EndIf}
  ${EndIf}
SectionEnd
!macroend

; Ticks the installed apps, or all on a first install.
!macro ElyFound NAME SLUG SHARES
  !insertmacro ElyKey ${NAME}
  ${If} $1 != ""
    StrCpy $0 "found"
  ${EndIf}
!macroend
!macro ElyTick NAME SLUG SHARES
  !insertmacro ElyKey ${NAME}
  ${If} $1 == ""
    !insertmacro UnselectSection $Index
  ${EndIf}
  IntOp $Index $Index + 1
!macroend

; An installed app left unticked is removed.
!macro ElyUntick NAME SLUG SHARES
  ${IfNot} ${SectionIsSelected} $Index
    !insertmacro ElyKey ${NAME}
    ${If} $1 != ""
      StrCpy $AppName "${NAME}"
      StrCpy $INSTDIR $1
      Call ElyRemove
      !insertmacro ElyUnlink ${SHARES}
      !insertmacro ElyForget ${NAME}
    ${EndIf}
  ${EndIf}
  IntOp $Index $Index + 1
!macroend

; One setup per user at a time; 128 is abandoned.
!macro ElyAlone
  ReadEnvStr $1 USERNAME
  System::Call 'kernel32::CreateMutexW(p 0, i 0, w "Global\ElyOfficeSetup-$1") p .r0'
  StrCpy $1 "null"
  ${If} $0 P<> 0
    System::Call 'kernel32::WaitForSingleObject(p r0, i -1) i .r1'
  ${EndIf}
  ${If} $1 != 0
  ${AndIf} $1 != 128
    MessageBox MB_ICONSTOP "$(Alone)$\r$\n$1" /SD IDOK
    SetErrorLevel 2
    Quit
  ${EndIf}
!macroend

; An installed suite has its root; no folder page.
Function ElyWhere
  ${If} $Suite == "yes"
    Abort
  ${EndIf}
FunctionEnd

Function .onInit
  InitPluginsDir
  !insertmacro ElyAlone
  ReadRegStr $1 HKCU "Software\Ely Office" "Root"
  ReadRegStr $2 HKCU "${UNINSTALL}\Ely Office" "UninstallString"
  ${If} $1 != ""
  ${AndIf} $2 == '"$1\uninstall.exe"'
  ${AndIf} ${FileExists} "$1\uninstall.exe"
    StrCpy $Suite "yes"
  ${EndIf}
  ; An app's own folder, as the old updater passes.
  ${If} ${FileExists} "$INSTDIR\.ely-installed"
    ${If} $Suite == "yes"
      StrCpy $INSTDIR $1
    ${Else}
      ${GetParent} "$INSTDIR" $INSTDIR
    ${EndIf}
  ${EndIf}
  ; An installed suite updates only where it is.
  ${If} $Suite == "yes"
  ${AndIf} $INSTDIR != $1
    MessageBox MB_ICONSTOP "$(Rooted)$\r$\n$1" /SD IDOK
    SetErrorLevel 2
    Quit
  ${EndIf}
  StrCpy $0 ""
  !insertmacro ElyAll ElyFound
  ${If} $0 == "found"
    StrCpy $Index 2
    !insertmacro ElyAll ElyTick
  ${EndIf}
  Call ElyKernel
FunctionEnd

; First: an app that fails later still has the entry.
Section "-Root"
  StrCpy $Root $INSTDIR
  ; Ours only if our entry runs it.
  ReadRegStr $0 HKCU "${UNINSTALL}\Ely Office" "UninstallString"
  ${If} ${FileExists} "$Root\uninstall.exe"
  ${AndIf} $0 != '"$Root\uninstall.exe"'
    StrCpy $AppName "Ely Office"
    MessageBox MB_ICONSTOP "$(Taken)$\r$\n$Root\uninstall.exe" /SD IDOK
    SetErrorLevel 2
    Quit
  ${EndIf}
  !insertmacro ElyStop
  ; Entry first: a kill leaves no orphan uninstaller.
  ClearErrors
  WriteRegStr HKCU "Software\Ely Office" "Root" "$Root"
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "DisplayName" "Ely Office"
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "Publisher" "ZacharyZhang"
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "URLInfoAbout" "https://zacharyzhang.com"
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "DisplayIcon" "$Root\uninstall.exe"
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "UninstallString" '"$Root\uninstall.exe"'
  ; Modify opens the same ticks, to remove apps.
  WriteRegStr HKCU "${UNINSTALL}\Ely Office" "ModifyPath" '"$Root\uninstall.exe"'
  SetOutPath "$Root"
  WriteUninstaller "$Root\uninstall.exe"
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "$(Entry)$\r$\n$Root" /SD IDOK
    Abort
  ${EndIf}
SectionEnd

; An app's own folder is never the kernel.
!macro ElyClash NAME SLUG SHARES
  !insertmacro ElyKey ${NAME}
  ${If} $1 == "$Root\Kernel"
    StrCpy $AppName "Ely Office"
    MessageBox MB_ICONSTOP "$(Taken)$\r$\n$1" /SD IDOK
    SetErrorLevel 2
    Quit
  ${EndIf}
!macroend

; Hidden: present while a document app is ticked.
Section "-Kernel" Kernel
  !insertmacro ElyAll ElyClash
  StrCpy $AppName "Ely Office"
  StrCpy $AppSlug "kernel"
  StrCpy $INSTDIR "$Root\Kernel"
  SetOutPath "$PLUGINSDIR"
  File "/oname=new.txt" "${STAGES}\kernel-files.txt"
  Call ElyPrepare
  StrCpy $R8 "copying"
  ClearErrors
  SetOutPath "$INSTDIR"
  File /r "${STAGES}\kernel\*.*"
  CopyFiles /SILENT "$PLUGINSDIR\new.txt" "$INSTDIR\.ely-installed"
  ${If} ${Errors}
    Abort
  ${EndIf}
  Call ElyEmptied
  StrCpy $R8 ""
  Call ElyPlaced
SectionEnd

!macro ElyNeed NAME SLUG SHARES
  !if ${SHARES} == 1
    ${If} ${SectionIsSelected} $Index
      StrCpy $0 "need"
    ${EndIf}
  !endif
  IntOp $Index $Index + 1
!macroend

; Ticks the kernel to match the document apps.
Function ElyKernel
  StrCpy $0 ""
  StrCpy $Index 2
  !insertmacro ElyAll ElyNeed
  ${If} $0 == "need"
    !insertmacro SelectSection ${Kernel}
  ${Else}
    !insertmacro UnselectSection ${Kernel}
  ${EndIf}
FunctionEnd

Function .onSelChange
  Call ElyKernel
FunctionEnd

; Uninstall's first: engines hold the kernel.
Section "-un.Stop"
  !insertmacro ElyStop
SectionEnd

!insertmacro ElyAll ElyApp

; No app left: no entry, no uninstaller.
!macro ElyLast
  StrCpy $0 ""
  !insertmacro ElyAll ElyFound
  ${If} $0 == ""
    StrCpy $AppName "Ely Office"
    ; The entry proves the file ours; it outlives it.
    Delete "$Root\uninstall.exe"
    ${If} ${FileExists} "$Root\uninstall.exe"
      MessageBox MB_ICONSTOP "$(Unkeyed)$\r$\n$Root\uninstall.exe" /SD IDOK
      SetErrorLevel 2
    ${Else}
      RMDir "$Root"
      !insertmacro ElyUnkey "${UNINSTALL}\Ely Office"
      ${If} $0 != 0
        MessageBox MB_ICONSTOP "$(Unkeyed)$\r$\nHKCU\${UNINSTALL}\Ely Office ($0)" /SD IDOK
        SetErrorLevel 2
      ${EndIf}
      System::Call 'advapi32::RegDeleteKeyValueW(p 0x80000001, w "Software\Ely Office", w "Root") i .r0'
      ${If} $0 != 0
      ${AndIf} $0 != 2
        MessageBox MB_ICONSTOP "$(Unkeyed)$\r$\nHKCU\Software\Ely Office ($0)" /SD IDOK
        SetErrorLevel 2
      ${EndIf}
      DeleteRegKey /ifempty HKCU "Software\Ely Office"
    ${EndIf}
  ${EndIf}
!macroend

Section "-Entry"
  StrCpy $Index 2
  !insertmacro ElyAll ElyUntick
  !insertmacro ElyUnkernel ""
  !insertmacro ElyWarm
  !insertmacro ElyLast
SectionEnd

; The uninstaller lists only installed apps.
!macro ElyHide NAME SLUG SHARES
  !insertmacro ElyKey ${NAME}
  ${If} $1 == ""
    !insertmacro UnselectSection $Index
    SectionSetText $Index ""
  ${EndIf}
  IntOp $Index $Index + 1
!macroend

Function un.onInit
  InitPluginsDir
  !insertmacro ElyAlone
  StrCpy $Root $INSTDIR
  StrCpy $Index 1
  !insertmacro ElyAll ElyHide
FunctionEnd

Section "-un.Entry"
  !insertmacro ElyUnkernel "un."
  !insertmacro ElyWarm
  !insertmacro ElyLast
SectionEnd
