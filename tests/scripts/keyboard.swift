// The ASCII keyboard layout chosen, or the given source restored.
import Carbon

func name(_ source: TISInputSource) -> String {
    let id = TISGetInputSourceProperty(source, kTISPropertyInputSourceID)!
    return Unmanaged<CFString>.fromOpaque(id).takeUnretainedValue() as String
}

let was = name(TISCopyCurrentKeyboardInputSource().takeRetainedValue())
var want = TISCopyCurrentASCIICapableKeyboardLayoutInputSource().takeRetainedValue()
if CommandLine.arguments.count > 1 {
    let filter = [kTISPropertyInputSourceID as String: CommandLine.arguments[1]] as CFDictionary
    guard let found = TISCreateInputSourceList(filter, false)?.takeRetainedValue() as? [TISInputSource],
          let source = found.first else {
        fputs("no input source \(CommandLine.arguments[1])\n", stderr)
        exit(1)
    }
    want = source
}
let status = TISSelectInputSource(want)
guard status == noErr else {
    fputs("selecting \(name(want)) failed: \(status)\n", stderr)
    exit(1)
}
// Read back: the source in use now.
print(was, name(TISCopyCurrentKeyboardInputSource().takeRetainedValue()))
