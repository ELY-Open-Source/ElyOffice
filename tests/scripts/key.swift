// Posts one key to a process; prints when it went.
import CoreGraphics
import Foundation

// key <pid> <virtual key code>
let pid = pid_t(CommandLine.arguments[1])!
let code = CGKeyCode(CommandLine.arguments[2])!
let source = CGEventSource(stateID: .privateState)
let sent = Date().timeIntervalSince1970 * 1000
for down in [true, false] {
    CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)!.postToPid(pid)
    usleep(30_000)
}
print("sent \(Int(sent))")
