#!/usr/bin/env bash
# Prints NSIS macros registering <slug>'s types, never as default.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
slug=${1:?app slug, such as ely-doc}
name=$(awk -F'\t' -v slug="$slug" '$1 == slug { print $2 }' "$root/packaging/apps.tsv")
[ -n "$name" ] || { echo "types.sh: unknown app $slug" >&2; exit 1; }
progid="ElyOffice.$name"
classes='Software\Classes'
command="'\"\$INSTDIR\\$slug.exe\" \"%1\"'"
exts=$(awk -F'\t' -v slug="$slug" '$1 == slug && $2 != "/" { print $2 }' "$root/packaging/types.tsv")
folder=$(awk -F'\t' -v slug="$slug" '$1 == slug && $2 == "/" { print "yes" }' "$root/packaging/types.tsv")
# Types Explorer's New menu makes, from packaging/windows/new.
fresh=""
for ext in $exts; do
  [ ! -e "$root/packaging/windows/new/new.$ext" ] || fresh="$fresh $ext"
done

# $2 keeps the first key that failed.
blame() { # indent key
  echo "$1  StrCmp \$2 \"\" 0 +2"
  echo "$1    StrCpy \$2 \"HKCU\\$2\""
}
# Writes one value, checked.
put() { # indent key name value
  echo "$1ClearErrors"
  echo "$1WriteRegStr HKCU \"$2\" \"$3\" $4"
  echo "$1IfErrors 0 +3"
  blame "$1" "$2"
}
# Only "not found" (2) is absence; denied reads fail.
absent() { # indent key [value]
  echo "$1System::Call 'advapi32::RegOpenKeyExW(i 0x80000001, w \"$2\", i 0, i 0x20019, *p .r3) i .r5'"
  echo "$1StrCpy \$4 \$5"
  echo "$1\${If} \$5 == 0"
  [ $# -lt 3 ] || echo "$1  System::Call 'advapi32::RegQueryValueExW(p r3, w \"$3\", p 0, p 0, p 0, p 0) i .r4'"
  echo "$1  System::Call 'advapi32::RegCloseKey(p r3)'"
  echo "$1\${EndIf}"
  echo "$1\${If} \$4 != 2"
  echo "$1\${AndIf} \$2 == \"\""
  echo "$1  StrCpy \$2 \"HKCU\\$2\""
  echo "$1\${EndIf}"
}
# Deletes one value; one still there failed.
drop() { # indent key name
  echo "$1DeleteRegValue HKCU \"$2\" \"$3\""
  absent "$1" "$2" "$3"
}
# Deletes one key; one still there failed.
gone() { # indent key
  echo "$1DeleteRegKey HKCU \"$2\""
  absent "$1" "$2"
}
# Reads who owns a registration; unreadable is a failure.
owner() { # indent key
  echo "$1ClearErrors"
  echo "$1ReadRegStr \$0 HKCU \"$2\" \"\""
  echo "$1\${If} \${Errors}"
  absent "$1  " "$2" ""
  echo "$1\${EndIf}"
}

# Open With lists the app; the user's default stays theirs.
echo '!macro ElyTypesInstall'
echo '  StrCpy $2 ""'
if [ -n "$exts" ]; then
  put "  " "$classes\\$progid" "" "\"$name Document\""
  put "  " "$classes\\$progid\\DefaultIcon" "" "\"\$INSTDIR\\$slug.ico\""
  put "  " "$classes\\$progid\\shell\\open\\command" "" "$command"
  put "  " "$classes\\Applications\\$slug.exe" "FriendlyAppName" "\"$name\""
  # Lists point at the class only once it opens.
  echo "  \${If} \$2 == \"\""
  for ext in $exts; do
    put "    " "$classes\\.$ext\\OpenWithProgids" "$progid" '""'
    put "    " "$classes\\Applications\\$slug.exe\\SupportedTypes" ".$ext" '""'
  done
  echo "  \${EndIf}"
fi
for ext in $fresh; do
  # New needs a class; take only a handlerless type.
  echo "  \${If} \$2 == \"\""
  echo "    ClearErrors"
  echo "    ReadRegStr \$0 HKCR \".$ext\" \"\""
  # A default naming no class handles nothing either.
  echo "    \${If} \$0 != \"\""
  echo "      EnumRegKey \$1 HKCR \"\$0\" 0"
  echo "    \${EndIf}"
  echo "    \${If} \$0 == \"\""
  echo "    \${OrIf} \${Errors}"
  echo "    \${OrIf} \$0 == \"$progid\""
  put "      " "$classes\\.$ext" "" "\"$progid\""
  put "      " "$classes\\.$ext\\ShellNew" "FileName" "\"\$INSTDIR\\new.$ext\""
  echo "    \${EndIf}"
  echo "  \${EndIf}"
done
if [ -n "$folder" ]; then
  put "  " "$classes\\Directory\\shell\\$progid" "" "\"Open in $name\""
  put "  " "$classes\\Directory\\shell\\$progid" "Icon" "\"\$INSTDIR\\$slug.ico\""
  put "  " "$classes\\Directory\\shell\\$progid\\command" "" "$command"
fi
echo '  System::Call "shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)"'
echo '  ClearErrors'
echo '  StrCmp $2 "" +2'
echo '    SetErrors'
echo '!macroend'

# Only what this install wrote and still owns.
echo '!macro ElyTypesUninstall'
echo '  StrCpy $2 ""'
echo "  StrCpy \$1 $command"
if [ -n "$exts" ]; then
  owner "  " "$classes\\$progid\\shell\\open\\command"
  echo "  \${If} \$0 == \$1"
  for ext in $exts; do
    drop "    " "$classes\\.$ext\\OpenWithProgids" "$progid"
  done
  for ext in $fresh; do
    echo "    ReadRegStr \$0 HKCU \"$classes\\.$ext\\ShellNew\" \"FileName\""
    echo "    \${If} \$0 == \"\$INSTDIR\\new.$ext\""
    drop "      " "$classes\\.$ext\\ShellNew" "FileName"
    echo "      DeleteRegKey /ifempty HKCU \"$classes\\.$ext\\ShellNew\""
    echo "    \${EndIf}"
    echo "    ReadRegStr \$0 HKCU \"$classes\\.$ext\" \"\""
    echo "    \${If} \$0 == \"$progid\""
    drop "      " "$classes\\.$ext" ""
    echo "    \${EndIf}"
  done
  # The class goes last, once nothing names it.
  echo "    \${If} \$2 == \"\""
  gone "      " "$classes\\Applications\\$slug.exe"
  echo "    \${EndIf}"
  echo "    \${If} \$2 == \"\""
  gone "      " "$classes\\$progid"
  echo "    \${EndIf}"
  echo "  \${EndIf}"
fi
if [ -n "$folder" ]; then
  owner "  " "$classes\\Directory\\shell\\$progid\\command"
  echo "  \${If} \$0 == \$1"
  gone "    " "$classes\\Directory\\shell\\$progid"
  echo "  \${EndIf}"
fi
echo '  System::Call "shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)"'
echo '  ClearErrors'
echo '  StrCmp $2 "" +2'
echo '    SetErrors'
echo '!macroend'
