pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "tauri-plugin-renovador-bg"

// Copiado pelo build.rs do crate (`tauri-plugin` build) ao compilar o crate.
include(":tauri-android")
project(":tauri-android").projectDir = file("./.tauri/tauri-api")
