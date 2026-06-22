import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

let size = 1024
let cs = CGColorSpaceCreateDeviceRGB()
guard let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8,
                          bytesPerRow: 0, space: cs,
                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
    fatalError("ctx")
}
let W = CGFloat(size)
func rr(_ r: CGRect, _ rad: CGFloat) -> CGPath {
    CGPath(roundedRect: r, cornerWidth: rad, cornerHeight: rad, transform: nil)
}

// ── 背景:深色 squircle + 竖向渐变 ──
ctx.saveGState()
ctx.addPath(rr(CGRect(x: 0, y: 0, width: W, height: W), W * 0.2237))
ctx.clip()
let grad = CGGradient(colorsSpace: cs,
    colors: [CGColor(red: 0.20, green: 0.20, blue: 0.22, alpha: 1),
             CGColor(red: 0.10, green: 0.10, blue: 0.12, alpha: 1)] as CFArray,
    locations: [0, 1])!
ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: W), end: CGPoint(x: 0, y: 0), options: [])
ctx.restoreGState()

// ── 两条用量进度条(沿用卡片配色:Claude 橙 / GPT 绿)──
func bar(y: CGFloat, frac: CGFloat, color: CGColor) {
    let barX: CGFloat = 270, barW: CGFloat = 540, barH: CGFloat = 104
    ctx.addPath(rr(CGRect(x: barX, y: y, width: barW, height: barH), barH / 2))
    ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 0.13)); ctx.fillPath()
    ctx.addPath(rr(CGRect(x: barX, y: y, width: barW * frac, height: barH), barH / 2))
    ctx.setFillColor(color); ctx.fillPath()
    let dotR: CGFloat = 34
    ctx.addEllipse(in: CGRect(x: barX - 96, y: y + barH / 2 - dotR, width: dotR * 2, height: dotR * 2))
    ctx.setFillColor(color); ctx.fillPath()
}
bar(y: 556, frac: 0.72, color: CGColor(red: 0xD9/255.0, green: 0x77/255.0, blue: 0x57/255.0, alpha: 1)) // Claude 橙
bar(y: 368, frac: 0.46, color: CGColor(red: 0x10/255.0, green: 0xA3/255.0, blue: 0x7F/255.0, alpha: 1)) // GPT 绿

let img = ctx.makeImage()!
let url = URL(fileURLWithPath: "/tmp/usagebar_icon.png")
let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(dest, img, nil)
if CGImageDestinationFinalize(dest) { print("wrote \(url.path)") } else { fatalError("write") }
