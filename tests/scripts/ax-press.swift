// Presses the first element named `title`, maybe of `role`.
import ApplicationServices
import Foundation

let pid = pid_t(CommandLine.arguments[1])!
let title = CommandLine.arguments[2]
let role = CommandLine.arguments.count > 3 ? CommandLine.arguments[3] : nil
guard AXIsProcessTrusted() else {
    print("untrusted")
    exit(0)
}
func attr(_ e: AXUIElement, _ n: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(e, n as CFString, &v) == .success ? v : nil
}
func find(_ e: AXUIElement, _ depth: Int) -> AXUIElement? {
    if depth > 40 { return nil }
    let named = (attr(e, kAXTitleAttribute) as? String) == title || (attr(e, kAXDescriptionAttribute) as? String) == title
    if named && (role == nil || (attr(e, kAXRoleAttribute) as? String) == role) {
        return e
    }
    for child in attr(e, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
        if let found = find(child, depth + 1) { return found }
    }
    return nil
}
let app = AXUIElementCreateApplication(pid)
// Dialogs can be windows the app's children leave out.
let windows = attr(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
let roots = [app] + windows
guard let target = roots.lazy.compactMap({ find($0, 0) }).first else {
    print("none")
    exit(0)
}
print(AXUIElementPerformAction(target, kAXPressAction as CFString) == .success ? "pressed" : "refused")
