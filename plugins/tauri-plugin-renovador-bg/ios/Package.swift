// swift-tools-version:5.3

import PackageDescription

let package = Package(
    name: "tauri-plugin-renovador-bg",
    platforms: [
        .macOS(.v10_13),
        .iOS(.v13),
    ],
    products: [
        .library(
            name: "tauri-plugin-renovador-bg",
            type: .static,
            targets: ["tauri-plugin-renovador-bg"]
        ),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
    ],
    targets: [
        .target(
            name: "tauri-plugin-renovador-bg",
            dependencies: [
                .byName(name: "Tauri"),
            ],
            path: "Sources"
        ),
    ]
)
