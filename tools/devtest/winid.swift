// Usage: winid PID  (prints that process's window ids, on-screen ones first and largest first within each, for screencapture -l)
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]) else {
  FileHandle.standardError.write("usage: winid PID\n".data(using: .utf8)!)
  exit(2)
}
let list = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as! [[String: Any]]
// WebKit keeps an off-screen helper window per process, often larger than a small real one, so
// windows that are on screen come first.
var found: [(id: Int, onscreen: Bool, area: Double)] = []
for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
  let layer = w[kCGWindowLayer as String] as? Int ?? -1
  let b = w[kCGWindowBounds as String] as? [String: Any] ?? [:]
  let area = (b["Height"] as? Double ?? 0) * (b["Width"] as? Double ?? 0)
  let onscreen = w[kCGWindowIsOnscreen as String] as? Bool ?? false
  if layer == 0 && area > 10_000 { found.append((w[kCGWindowNumber as String] as! Int, onscreen, area)) }
}
found.sort { ($0.onscreen ? 1 : 0, $0.area) > ($1.onscreen ? 1 : 0, $1.area) }
for w in found { print(w.id) }
