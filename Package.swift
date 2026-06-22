// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "TokenUsageDashboard",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "TokenUsageDashboard",
            path: "Sources/UsageBar",
            swiftSettings: [
                .unsafeFlags(["-parse-as-library"])
            ]
        )
    ]
)
