; One app's install: old files aside whole, or nothing changes.

; FileWrite; a failure marks $R5.
!macro ElyWrite HANDLE TEXT
  ClearErrors
  FileWrite ${HANDLE} "${TEXT}"
  IfErrors 0 +2
    StrCpy $R5 "unwritten"
!macroend

; A failed list: $R0 names none, so ElyAside refuses.
!macro ElyUnlisted
  Delete "$R0"
  StrCpy $R0 "$PLUGINSDIR\unlisted"
!macroend

; Lists a legacy folder's files and folders to $R2.
Function ElyWalk
  Push "|"
  Push $R1
walk:
  Pop $R1
  StrCmp $R1 "|" done
  !insertmacro ElyWrite $R2 "$R1\$\r$\n"
  FindFirst $R3 $R4 "$INSTDIR\$R1\*"
entry:
  StrCmp $R4 "" closed
  StrCmp $R4 "." skip
  StrCmp $R4 ".." skip
  ; Junctions stay out: .ely-old's RMDir /r follows them.
  System::Call 'kernel32::GetFileAttributesW(w "$INSTDIR\$R1\$R4") i .R6'
  IntOp $R6 $R6 & 0x410
  IntCmp $R6 0x10 0 file skip
  Push "$R1\$R4"
  Goto skip
file:
  !insertmacro ElyWrite $R2 "$R1\$R4$\r$\n"
skip:
  FindNext $R3 $R4
  Goto entry
closed:
  FindClose $R3
  Goto walk
done:
FunctionEnd

; A real folder only, never a junction.
!macro ElyLegacyDir NAME
  System::Call 'kernel32::GetFileAttributesW(w "$INSTDIR\${NAME}") i .R6'
  IntOp $R6 $R6 & 0x410
  IntCmp $R6 0x10 0 +3 +3
    StrCpy $R1 "${NAME}"
    Call ElyWalk
!macroend

!macro ElyLegacy ID PATTERN
  FindFirst $R3 $R4 "$INSTDIR\${PATTERN}"
  ely_legacy_${ID}:
    StrCmp $R4 "" ely_legacy_end_${ID}
    !insertmacro ElyWrite $R2 "$R4$\r$\n"
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
  Call ElyOwn
  Goto done
unlisted:
  ; Older releases: only the folder our key names.
  ReadRegStr $R1 HKCU "Software\Ely Office\$AppName" "InstallDir"
  StrCmp $R1 "$INSTDIR" 0 done
  IfFileExists "$INSTDIR\$AppSlug.exe" 0 done
  StrCpy $R0 "$PLUGINSDIR\legacy.txt"
  StrCpy $R5 ""
  ClearErrors
  FileOpen $R2 "$R0" w
  IfErrors 0 walk
    !insertmacro ElyUnlisted
    Goto done
walk:
  !insertmacro ElyLegacyDir libreoffice
  !insertmacro ElyLegacyDir tessdata
  !insertmacro ElyLegacyDir git
  !insertmacro ElyLegacyDir content
  !insertmacro ElyLegacyDir locales
  !insertmacro ElyLegacyDir licenses
  !insertmacro ElyWrite $R2 "$AppSlug.ico$\r$\n"
  !insertmacro ElyLegacy exe "*.exe"
  !insertmacro ElyLegacy dll "*.dll"
  !insertmacro ElyLegacy pak "*.pak"
  !insertmacro ElyLegacy bin "*.bin"
  !insertmacro ElyLegacy dat "*.dat"
  !insertmacro ElyLegacy json "*.json"
  FileClose $R2
  StrCmp $R5 "" done
  !insertmacro ElyUnlisted
done:
FunctionEnd

!macro ElyAside UN
; The list to $R0; a junction's files are the kernel's.
Function ${UN}ElyOwn
  System::Call 'kernel32::GetFileAttributesW(w "$INSTDIR\libreoffice") i .R3'
  StrCpy $R4 "own"
  IntCmp $R3 -1 copy
  IntOp $R3 $R3 & 0x400
  IntCmp $R3 0 copy
  StrCpy $R4 "kernel"
copy:
  StrCpy $R5 ""
  ClearErrors
  FileOpen $R2 "$INSTDIR\.ely-installed" r
  IfErrors failed
  FileOpen $R3 "$R0" w
  IfErrors 0 line
    FileClose $R2
    Goto failed
line:
  ClearErrors
  FileRead $R2 $R6
  IfErrors done
  StrCmp $R4 "own" keep
  StrCpy $8 "$R6" 12
  StrCmp $8 "libreoffice\" line
keep:
  !insertmacro ElyWrite $R3 "$R6"
  Goto line
done:
  ; A read error also stops FileRead; only the end counts.
  FileSeek $R2 0 CUR $8
  FileSeek $R2 0 END $9
  StrCmp $8 $9 +2
    StrCpy $R5 "unread"
  FileClose $R3
  FileClose $R2
  StrCmp $R5 "" 0 failed
  Return
failed:
  !insertmacro ElyUnlisted
FunctionEnd

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
  ; Pruned folders return; the leaf stays free.
  CreateDirectory "$INSTDIR\$R3"
  RMDir "$INSTDIR\$R3"
  Rename "$R9\$9" "$INSTDIR\$R3"
  Goto back
done:
  FileClose $R2
  RMDir "$R9"
  IfFileExists "$R9" 0 +2
    MessageBox MB_ICONSTOP "$(Stuck)$\r$\n$R9" /SD IDOK
FunctionEnd

; Removes what the list at $INSTDIR names; busy changes nothing.
Function ${UN}ElyRemove
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
  StrCpy $R0 "$PLUGINSDIR\owned.txt"
  Call ${UN}ElyOwn
  Call ${UN}ElyAside
  StrCmp $R5 "" gone busy
gone:
  ClearErrors
  Delete "$INSTDIR\.ely-installed"
  IfErrors 0 deleted
  Call ${UN}ElyBack
  StrCpy $R5 "$INSTDIR\.ely-installed"
busy:
  MessageBox MB_ICONSTOP "$(Busy)$\r$\n$R5" /SD IDOK
  SetErrorLevel 2
  Quit
deleted:
  RMDir /r "$INSTDIR\.ely-old"
  StrCpy $R1 "$INSTDIR"
  Call ${UN}ElyPrune
  RMDir "$INSTDIR"
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

; The old install aside, the new list clear; else quits.
Function ElyPrepare
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
  StrCmp $R5 "" done
  StrCmp $R0 "" +2
    Call ElyBack
  MessageBox MB_ICONSTOP "$(Taken)$\r$\n$R5" /SD IDOK
  SetErrorLevel 2
  Quit
done:
FunctionEnd

; Copied: old folders left empty go.
Function ElyEmptied
  StrCmp $R0 "" +3
    StrCpy $R1 "$INSTDIR"
    Call ElyPrune
FunctionEnd

; Committed: the backup goes.
Function ElyPlaced
  RMDir /r "$INSTDIR\.ely-old"
  IfFileExists "$INSTDIR\.ely-old" 0 +2
    MessageBox MB_ICONEXCLAMATION "$(Left)$\r$\n$INSTDIR\.ely-old" /SD IDOK
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
