// What VoiceOver reads: names, the page, and Tab's path.
import AppKit
import ApplicationServices
import Foundation

// ax-names <pid> <page name or empty>
let pid = pid_t(CommandLine.arguments[1])!
let page = CommandLine.arguments[2]
let acts: Set<String> = ["AXButton", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXComboBox", "AXTextField",
                         "AXTextArea", "AXMenuItem", "AXSlider", "AXIncrementor", "AXLink", "AXRow"]
let app = AXUIElementCreateApplication(pid)

func attr(_ e: AXUIElement, _ n: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(e, n as CFString, &v) == .success ? v : nil
}
func text(_ e: AXUIElement, _ n: String) -> String { attr(e, n) as? String ?? "" }
func role(_ e: AXUIElement) -> String { text(e, kAXRoleAttribute) }
func named(_ e: AXUIElement) -> Bool {
    !text(e, kAXTitleAttribute).isEmpty || !text(e, kAXDescriptionAttribute).isEmpty
        || attr(e, kAXTitleUIElementAttribute) != nil
}
func isPage(_ e: AXUIElement) -> Bool {
    !page.isEmpty && text(e, kAXTitleAttribute) == page
        && (role(e) == "AXTextArea" || text(e, kAXSubroleAttribute) == "AXDocument")
}
func nodes(_ e: AXUIElement, _ depth: Int = 0) -> [AXUIElement] {
    if depth > 60 { return [] }
    return [e] + (attr(e, kAXChildrenAttribute) as? [AXUIElement] ?? []).flatMap { nodes($0, depth + 1) }
}
func key(_ e: AXUIElement) -> String {
    let parent = attr(e, kAXParentAttribute).map { $0 as! AXUIElement }
    let siblings = parent.flatMap { attr($0, kAXChildrenAttribute) as? [AXUIElement] } ?? []
    let index = siblings.firstIndex { CFEqual($0, e) } ?? -1
    return "\(role(e))|\(text(e, kAXTitleAttribute))|\(index)|\(parent.map { text($0, kAXTitleAttribute) } ?? "")"
}
func focused() -> AXUIElement? { attr(app, kAXFocusedUIElementAttribute).map { $0 as! AXUIElement } }
// The system-wide focused app reads as none at times.
func active() -> Bool { NSRunningApplication(processIdentifier: pid)?.isActive ?? false }
func press(_ code: CGKeyCode) -> AXUIElement? {
    let source = CGEventSource(stateID: .privateState)
    for down in [true, false] {
        CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)!.postToPid(pid)
        usleep(30_000)
    }
    usleep(400_000)
    return focused()
}

guard AXIsProcessTrusted() else {
    print("untrusted")
    exit(2)
}
var log: [String] = []
// A first read wakes the tree; the app sends it.
_ = (attr(app, kAXWindowsAttribute) as? [AXUIElement] ?? []).map { nodes($0) }
usleep(1_500_000)
let windows = attr(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
let all = ([app] + windows).flatMap { nodes($0) }
// Separators, and the window buttons macOS names itself.
let system: Set<String> = ["AXCloseButton", "AXMinimizeButton", "AXFullScreenButton", "AXZoomButton"]
func separator(_ e: AXUIElement) -> Bool {
    role(e) == "AXMenuItem" && text(e, kAXTitleAttribute).isEmpty && (attr(e, kAXEnabledAttribute) as? Bool) == false
}
for e in all where acts.contains(role(e)) && !named(e) && !separator(e) && !system.contains(text(e, kAXSubroleAttribute)) {
    log.append("unnamed \(role(e)) \(text(e, kAXSubroleAttribute))")
}
if !page.isEmpty && !all.contains(where: isPage) { log.append("no page [\(page)]") }
// The last app's window may still be closing.
for _ in 0..<10 where !active() {
    NSRunningApplication(processIdentifier: pid)?.activate()
    usleep(500_000)
}
if !active() { log.append("error the app never came to the front") }
// Tab 48, F6 97.
var seen = Set<String>(), last = "", stops = 0, reached = false
for _ in 0..<150 {
    guard var node = press(48) else { log.append("no focus after stop \(stops)"); break }
    // A document keeps Tab; F6 leaves it.
    if key(node) == last {
        guard let left = press(97) else { log.append("no focus after stop \(stops)"); break }
        node = left
        if key(node) == last { log.append("trapped in \(last)"); break }
    }
    if !active() { log.append("error focus left the app at stop \(stops)"); break }
    let k = key(node)
    if seen.contains(k) { break }
    seen.insert(k)
    last = k
    stops += 1
    log.append("stop \(stops) \(k)")
    if !named(node) { log.append("nameless stop \(stops) \(role(node))") }
    var up: AXUIElement? = node
    while let at = up, !reached {
        reached = isPage(at)
        up = attr(at, kAXParentAttribute).map { $0 as! AXUIElement }
    }
}
// Tab must lead back to the page.
if !page.isEmpty && !reached { log.append("unreached page [\(page)]") }
log.append("stops \(stops)")
print(log.joined(separator: "\n"))
