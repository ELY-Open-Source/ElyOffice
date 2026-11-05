// Middle shade, grid-line share, title bar shade.
import CoreGraphics
import Foundation
import ImageIO

let pid = Int32(CommandLine.arguments[1])!
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
guard let window = windows.first(where: {
    ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int) == 0
}) else {
    print("none")
    exit(0)
}
let id = window[kCGWindowNumber as String] as! Int
let file = NSTemporaryDirectory() + "ely-shade-\(pid).png"
let shot = Process()
shot.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
shot.arguments = ["-x", "-o", "-l", "\(id)", file]
try shot.run()
shot.waitUntilExit()
guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: file) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    print("denied")
    exit(0)
}
let (width, height) = (image.width, image.height)
var pixels = [UInt8](repeating: 0, count: width * height * 4)
let context = CGContext(
    data: &pixels, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
    space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
// Mean shade, and the share off the common pixel.
var shades = [Int]()
for y in (height * 2 / 5)..<(height * 7 / 10) {
    for x in (width * 3 / 10)..<(width * 7 / 10) {
        let at = (y * width + x) * 4
        shades.append((Int(pixels[at]) + Int(pixels[at + 1]) + Int(pixels[at + 2])) / 3)
    }
}
var tally = [Int](repeating: 0, count: 256)
for shade in shades { tally[shade] += 1 }
let common = tally.firstIndex(of: tally.max()!)!
let lines = shades.filter { abs($0 - common) > 12 }.count * 1000 / shades.count
// The title bar, clear of buttons and title.
var frame = [Int]()
for y in (height / 100)..<(height * 3 / 100) {
    for x in (width / 5)..<(width * 2 / 5) {
        let at = (y * width + x) * 4
        frame.append((Int(pixels[at]) + Int(pixels[at + 1]) + Int(pixels[at + 2])) / 3)
    }
}
print("\(shades.reduce(0, +) / shades.count) \(lines) \(frame.reduce(0, +) / frame.count)")
