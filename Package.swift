// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "CSUSTKitRust",
    platforms: [.macOS(.v14), .iOS(.v17)],
    products: [
        .library(name: "CSUSTKitRust", targets: ["CSUSTKitRust"])
    ],
    targets: [
        .target(
            name: "CSUSTKitRust",
            dependencies: ["CSUSTKitRustFFI"],
            path: "swift/Generated",
            linkerSettings: [
                .linkedFramework("CoreFoundation"),
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
            ]
        ),
        .binaryTarget(
            name: "CSUSTKitRustFFI",
            path: "swift/Artifacts/CSUSTKitRustFFI.xcframework"
        ),
    ]
)
