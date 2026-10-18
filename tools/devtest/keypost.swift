// Usage: keypost PID KEYCODE [cmd] [shift] [alt] [ctrl]  (one key press delivered to that process only)
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]), let code = CGKeyCode(args[2]) else {
  FileHandle.standardError.write("usage: keypost PID KEYCODE [cmd] [shift] [alt] [ctrl]\n".data(using: .utf8)!)
  exit(2)
}
var flags: CGEventFlags = []
for a in args.dropFirst(3) {
  switch a {
  case "cmd": flags.insert(.maskCommand)
  case "shift": flags.insert(.maskShift)
  case "alt": flags.insert(.maskAlternate)
  case "ctrl": flags.insert(.maskControl)
  default: break
  }
}
let src = CGEventSource(stateID: .privateState)
for down in [true, false] {
  let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down)!
  e.flags = flags
  e.postToPid(pid)
  usleep(30_000)
}
