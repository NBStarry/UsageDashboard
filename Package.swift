// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "UsageBar",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "UsageBar",
            path: "Sources/UsageBar",
            swiftSettings: [
                .unsafeFlags(["-parse-as-library"])
            ]
        )
    ]
)
