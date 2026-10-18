// Usage: px PNG X Y  (brightness 0..1, the brightest within 3px of point X,Y in screenshot points; a focus ring reads near 0.9)
import AppKit

let args = CommandLine.arguments
guard args.count == 4, let img = NSImage(contentsOfFile: args[1]),
      let tiff = img.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
      let px = Double(args[2]), let py = Double(args[3]) else {
  FileHandle.standardError.write("usage: px PNG X Y\n".data(using: .utf8)!)
  exit(2)
}
let scale = Double(rep.pixelsWide) / Double(img.size.width)
var best = 0.0
for dx in -3...3 {
  for dy in -3...3 {
    if let c = rep.colorAt(x: Int(px * scale) + dx, y: Int(py * scale) + dy) {
      best = max(best, c.brightnessComponent)
    }
  }
}
print(String(format: "%.2f", best))
