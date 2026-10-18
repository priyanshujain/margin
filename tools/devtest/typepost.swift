// Usage: typepost PID TEXT  (types TEXT as unicode key events delivered to that process only)
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count == 3, let pid = pid_t(args[1]) else {
  FileHandle.standardError.write("usage: typepost PID TEXT\n".data(using: .utf8)!)
  exit(2)
}
let src = CGEventSource(stateID: .privateState)
for ch in args[2].utf16 {
  var c = ch
  for down in [true, false] {
    let e = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: down)!
    e.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c)
    e.postToPid(pid)
    usleep(12_000)
  }
}
