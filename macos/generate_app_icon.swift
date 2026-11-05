import AppKit
import Foundation

guard CommandLine.arguments.count == 2 else {
    fputs("Usage: generate_app_icon.swift OUTPUT.iconset\n", stderr)
    exit(2)
}

let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)

let iconSizes: [(filename: String, pixels: Int)] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]

func color(_ red: CGFloat, _ green: CGFloat, _ blue: CGFloat, _ alpha: CGFloat = 1) -> NSColor {
    NSColor(calibratedRed: red / 255, green: green / 255, blue: blue / 255, alpha: alpha)
}

func drawIcon() {
    let background = NSBezierPath(roundedRect: NSRect(x: 20, y: 20, width: 472, height: 472), xRadius: 112, yRadius: 112)
    let gradient = NSGradient(colors: [color(39, 67, 43), color(11, 24, 17)])!
    gradient.draw(in: background, angle: -90)

    let panel = NSBezierPath(roundedRect: NSRect(x: 54, y: 58, width: 404, height: 396), xRadius: 66, yRadius: 66)
    color(9, 20, 14).setFill()
    panel.fill()
    color(126, 207, 103).setStroke()
    panel.lineWidth = 5
    panel.stroke()

    color(162, 244, 127).setFill()
    NSBezierPath(ovalIn: NSRect(x: 82, y: 397, width: 14, height: 14)).fill()

    let header = NSAttributedString(
        string: "RUST-BOY",
        attributes: [
            .font: NSFont.monospacedSystemFont(ofSize: 28, weight: .semibold),
            .foregroundColor: color(181, 235, 155),
            .kern: 4,
        ]
    )
    header.draw(at: NSPoint(x: 111, y: 390))

    color(89, 143, 77).setFill()
    NSBezierPath(roundedRect: NSRect(x: 80, y: 360, width: 352, height: 3), xRadius: 1.5, yRadius: 1.5).fill()

    let monogram = NSAttributedString(
        string: "RB",
        attributes: [
            .font: NSFont(name: "Menlo-Bold", size: 154) ?? NSFont.monospacedSystemFont(ofSize: 154, weight: .bold),
            .foregroundColor: color(166, 244, 126),
            .kern: -12,
        ]
    )
    let monogramSize = monogram.size()
    monogram.draw(at: NSPoint(x: (512 - monogramSize.width) / 2, y: 158))

    for index in 0..<5 {
        let width = CGFloat(24 + index * 18)
        let x = CGFloat(104 + index * 62)
        color(91 + CGFloat(index * 10), 163 + CGFloat(index * 12), 78 + CGFloat(index * 8)).setFill()
        NSBezierPath(roundedRect: NSRect(x: x, y: 112, width: width, height: 7), xRadius: 3.5, yRadius: 3.5).fill()
    }
}

for icon in iconSizes {
    guard let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: icon.pixels,
        pixelsHigh: icon.pixels,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ), let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
        fatalError("Could not create bitmap for \(icon.filename)")
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.imageInterpolation = .high
    context.cgContext.scaleBy(x: CGFloat(icon.pixels) / 512, y: CGFloat(icon.pixels) / 512)
    drawIcon()
    NSGraphicsContext.restoreGraphicsState()

    let destination = output.appendingPathComponent(icon.filename)
    guard let png = bitmap.representation(using: .png, properties: [:]) else {
        fatalError("Could not encode \(icon.filename)")
    }
    try png.write(to: destination)
}
