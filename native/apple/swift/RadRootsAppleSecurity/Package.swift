// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "RadRootsAppleSecurity",
    platforms: [
        .iOS(.v17),
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "RadRootsAppleSecurity",
            targets: ["RadRootsAppleSecurity"]
        ),
        .library(
            name: "RadRootsAppleSecurityFFI",
            type: .static,
            targets: ["RadRootsAppleSecurityFFI"]
        ),
        .library(
            name: "RadRootsAppleSecurityFFIDynamic",
            type: .dynamic,
            targets: ["RadRootsAppleSecurityFFI"]
        )
    ],
    targets: [
        .target(
            name: "RadRootsAppleSecurity",
            path: "Sources/RadRootsAppleSecurity"
        ),
        .target(
            name: "RadRootsAppleSecurityFFI",
            dependencies: ["RadRootsAppleSecurity"],
            path: "Sources/RadRootsAppleSecurityFFI"
        ),
        .testTarget(
            name: "RadRootsAppleSecurityTests",
            dependencies: ["RadRootsAppleSecurity", "RadRootsAppleSecurityFFI"],
            path: "Tests/RadRootsAppleSecurityTests"
        )
    ]
)
