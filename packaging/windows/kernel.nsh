; The shared kernel: links to it, and its removal.

; $1 resolved through links, or "".
Function ElyFinal
  System::Call 'kernel32::CreateFileW(w r1, i 0, i 7, p 0, i 3, i 0x02000000, p 0) i .r3'
  StrCpy $1 ""
  ${If} $3 != -1
    System::Call 'kernel32::GetFinalPathNameByHandleW(i r3, w .r1, i ${NSIS_MAX_STRLEN}, i 0) i .r4'
    System::Call 'kernel32::CloseHandle(i r3)'
    ${If} $4 == 0
    ${OrIf} $4 >= ${NSIS_MAX_STRLEN}
      StrCpy $1 ""
    ${EndIf}
  ${EndIf}
FunctionEnd

; Links libreoffice to the kernel; anything else there refuses.
!macro ElyLink
  System::Call 'kernel32::GetFileAttributesW(w "$INSTDIR\libreoffice") i .r0'
  ${If} $0 == -1
    nsExec::ExecToStack '"$SYSDIR\cmd.exe" /c mklink /J "$INSTDIR\libreoffice" "$Root\Kernel"'
    Pop $0
    Pop $1
    ${If} $0 != 0
      MessageBox MB_ICONSTOP "$(Unlinked)$\r$\n$INSTDIR\libreoffice ($1)" /SD IDOK
      Abort
    ${EndIf}
  ${Else}
    ; A link elsewhere is never re-pointed.
    IntOp $0 $0 & 0x400
    StrCpy $1 "$INSTDIR\libreoffice"
    Call ElyFinal
    StrCpy $2 $1
    StrCpy $1 "$Root\Kernel"
    Call ElyFinal
    ${If} $0 == 0
    ${OrIf} $1 == ""
    ${OrIf} $1 != $2
      MessageBox MB_ICONSTOP "$(Taken)$\r$\n$INSTDIR\libreoffice" /SD IDOK
      Abort
    ${EndIf}
  ${EndIf}
!macroend

; RMDir, never /r: a junction's target would go.
!macro ElyUnlink SHARES
  !if ${SHARES} == 1
    RMDir "$INSTDIR\libreoffice"
    RMDir "$INSTDIR"
  !endif
!macroend

!macro ElyShares NAME SLUG SHARES
  !if ${SHARES} == 1
    !insertmacro ElyKey ${NAME}
    ${If} $1 != ""
      StrCpy $0 "shared"
    ${EndIf}
  !endif
!macroend

; No document app left: the kernel goes.
!macro ElyUnkernel UN
  StrCpy $0 ""
  !insertmacro ElyAll ElyShares
  ${If} $0 == ""
  ${AndIf} ${FileExists} "$Root\Kernel\program\ely-kernel-id"
    StrCpy $AppName "Ely Office"
    StrCpy $INSTDIR "$Root\Kernel"
    Call ${UN}ElyRemove
  ${EndIf}
!macroend
