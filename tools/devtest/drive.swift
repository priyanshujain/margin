// Usage: drive PID TOKEN...  (tokens: tab stab space enter esc up down left right text:STR cmd:KEYCODE cmdshift:KEYCODE wait:MS; activates that app by PID first, keys go to it only)
import AppKit
import CoreGraphics

let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]), let app = NSRunningApplication(processIdentifier: pid) else {
  FileHandle.standardError.write("usage: drive PID TOKEN...\n".data(using: .utf8)!)
  exit(2)
}
let src = CGEventSource(stateID: .privateState)

// WebKit only hands keys to the DOM of the key window, so the app is brought forward whenever
// something else has taken focus in the meantime.
func ensureActive() {
  if !app.isActive {
    app.activate()
    usleep(250_000)
  }
}

func key(_ code: CGKeyCode, _ flags: CGEventFlags = []) {
  ensureActive()
  for down in [true, false] {
    let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down)!
    e.flags = flags
    e.postToPid(pid)
    usleep(25_000)
  }
}

func text(_ s: String) {
  for ch in s.utf16 {
    ensureActive()
    var c = ch
    for down in [true, false] {
      let e = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: down)!
      e.keyboardSetUnicodeString(stringLength: 1, unicodeString: &c)
      e.postToPid(pid)
      usleep(12_000)
    }
  }
}

for t in args.dropFirst(2) {
  switch t {
  case "tab": key(48)
  case "stab": key(48, .maskShift)
  case "space": key(49)
  case "enter": key(36)
  case "esc": key(53)
  case "up": key(126)
  case "down": key(125)
  case "left": key(123)
  case "right": key(124)
  default:
    if t.hasPrefix("text:") { text(String(t.dropFirst(5))) }
    else if t.hasPrefix("cmdshift:"), let c = CGKeyCode(t.dropFirst(9)) { key(c, [.maskCommand, .maskShift]) }
    else if t.hasPrefix("cmd:"), let c = CGKeyCode(t.dropFirst(4)) { key(c, .maskCommand) }
    else if t.hasPrefix("wait:"), let ms = useconds_t(t.dropFirst(5)) { usleep(ms * 1000) }
    else { FileHandle.standardError.write("unknown token \(t)\n".data(using: .utf8)!); exit(2) }
  }
}
