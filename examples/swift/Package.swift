// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "CSUSTKitSwiftExample",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "csustkit-swift", targets: ["CSUSTKitCLI"])
    ],
    dependencies: [
        .package(name: "CSUSTKitRust", path: "../..")
    ],
    targets: [
        .executableTarget(
            name: "CSUSTKitCLI",
            dependencies: [.product(name: "CSUSTKitRust", package: "CSUSTKitRust")]
        )
    ]
)
