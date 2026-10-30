// Clicks the left button at x y in screen points.
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

// With a pid, clicks on other processes' windows are refused.
let point = CGPoint(x: Double(CommandLine.arguments[1])!, y: Double(CommandLine.arguments[2])!)
if CommandLine.arguments.count > 3 {
    let pid = pid_t(CommandLine.arguments[3])!
    var element: AXUIElement?
    let found = AXUIElementCopyElementAtPosition(AXUIElementCreateSystemWide(), Float(point.x), Float(point.y), &element)
    var owner: pid_t = 0
    if found != .success || element == nil || AXUIElementGetPid(element!, &owner) != .success {
        FileHandle.standardError.write("nothing readable at \(point) (\(found.rawValue)); no click sent\n".data(using: .utf8)!)
        exit(1)
    }
    if owner != pid {
        let name = NSRunningApplication(processIdentifier: owner)?.localizedName ?? "unknown"
        FileHandle.standardError.write("\(name) (\(owner)) covers \(point), not \(pid); no click sent\n".data(using: .utf8)!)
        exit(1)
    }
}
for kind in [CGEventType.leftMouseDown, .leftMouseUp] {
    CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: point, mouseButton: .left)!
        .post(tap: .cghidEventTap)
    usleep(60_000)
}
