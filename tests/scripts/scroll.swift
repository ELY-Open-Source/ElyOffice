// Scrolls a process's first window by wheel notches, paced.
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

// scroll <pid> <notches, negative up> <ms between notches>
let pid = pid_t(CommandLine.arguments[1])!
let notches = Int(CommandLine.arguments[2])!
let pace = useconds_t(Int(CommandLine.arguments[3])! * 1000)

func attribute(_ element: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
    return value
}

func fail(_ message: String) -> Never {
    FileHandle.standardError.write("\(message)\n".data(using: .utf8)!)
    exit(1)
}

guard AXIsProcessTrusted() else { fail("needs Accessibility permission") }
let app = AXUIElementCreateApplication(pid)
guard let window = (attribute(app, kAXWindowsAttribute) as? [AXUIElement])?.first,
      let position = attribute(window, kAXPositionAttribute),
      let extent = attribute(window, kAXSizeAttribute) else { fail("process \(pid) shows no window") }
var origin = CGPoint.zero, size = CGSize.zero
AXValueGetValue(position as! AXValue, .cgPoint, &origin)
AXValueGetValue(extent as! AXValue, .cgSize, &size)
let middle = CGPoint(x: origin.x + size.width / 2, y: origin.y + size.height / 2)
// A wheel posted to a pid is dropped; post system-wide.
var element: AXUIElement?
var owner: pid_t = 0
let found = AXUIElementCopyElementAtPosition(AXUIElementCreateSystemWide(), Float(middle.x), Float(middle.y), &element)
if found != .success || element == nil || AXUIElementGetPid(element!, &owner) != .success || owner != pid {
    let name = NSRunningApplication(processIdentifier: owner)?.localizedName ?? "nothing readable"
    fail("\(name) (\(owner)) covers \(middle), not \(pid); no wheel sent")
}
CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: middle, mouseButton: .left)!
    .post(tap: .cghidEventTap)
usleep(100_000)
let sent = Date().timeIntervalSince1970 * 1000
for _ in 0..<abs(notches) {
    let event = CGEvent(scrollWheelEvent2Source: nil, units: .line, wheelCount: 1,
                        wheel1: notches > 0 ? -3 : 3, wheel2: 0, wheel3: 0)!
    event.location = middle
    event.post(tap: .cghidEventTap)
    usleep(pace)
}
print("sent \(Int(sent))")
