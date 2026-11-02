; Installs the ticked apps, each by its own setup.
Unicode true
ManifestDPIAware true
ManifestDPIAwareness PerMonitorV2
!include "MUI2.nsh"

!ifndef VERSION
  !error "pass -DVERSION"
!endif
!ifndef APPS
  !error "pass -DAPPS, from packaging/windows/bundle.sh"
!endif
!ifndef OUTFILE
  !error "pass -DOUTFILE"
!endif

Name "Ely Office ${VERSION}"
OutFile "${OUTFILE}"
RequestExecutionLevel user
; New apps go here, each in its own folder.
InstallDir "$LOCALAPPDATA\Programs\Ely Office"
InstallDirRegKey HKCU "Software\Ely Office" "Root"
; The setups inside are compressed already.
SetCompress off

!define MUI_DIRECTORYPAGE_TEXT_TOP "$(Where)"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

LangString Where ${LANG_SIMPCHINESE} "新装的应用各放在此文件夹下自己的子文件夹里；已装的应用在原处升级。"
LangString Where ${LANG_ENGLISH} "New apps each go in their own folder inside this one; installed apps update where they are."
LangString Failed ${LANG_SIMPCHINESE} "$R0 未装上，其安装程序退出码为 $R1（2 表示文件被占用或安装位置被占）。之前装上的应用照旧可用。"
LangString Failed ${LANG_ENGLISH} "$R0 was not installed; its setup exited with $R1 (2: a file is in use or in the way). Apps installed before it work."

Function .onInit
  InitPluginsDir
FunctionEnd

; One section per app; its setup runs silent.
!macro ElyApp NAME SETUP
Section "${NAME}"
  SetOutPath "$PLUGINSDIR"
  File "/oname=${NAME}-setup.exe" "${SETUP}"
  DetailPrint "${NAME}"
  StrCpy $R0 "${NAME}"
  ; Installed: in place; new: under the chosen folder.
  ReadRegStr $R2 HKCU "Software\Ely Office\${NAME}" "InstallDir"
  StrCmp $R2 "" 0 +2
    StrCpy $R2 "$INSTDIR\${NAME}"
  ClearErrors
  ExecWait '"$PLUGINSDIR\${NAME}-setup.exe" /S /D=$R2' $R1
  IfErrors 0 +2
    StrCpy $R1 "-"
  Delete "$PLUGINSDIR\${NAME}-setup.exe"
  StrCmp $R1 0 ely_done_${NAME}
  MessageBox MB_ICONSTOP "$(Failed)" /SD IDOK
  SetErrorLevel 2
  Abort
  ely_done_${NAME}:
SectionEnd
!macroend

!include "${APPS}"

; Remembered for the next bundle.
Section "-Root"
  WriteRegStr HKCU "Software\Ely Office" "Root" "$INSTDIR"
SectionEnd
