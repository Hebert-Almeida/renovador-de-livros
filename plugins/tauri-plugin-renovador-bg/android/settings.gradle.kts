pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
    // Só para compilar o plugin sozinho; no app valem as versões do projeto.
    plugins {
        id("com.android.library") version "8.9.1"
        id("org.jetbrains.kotlin.android") version "2.1.0"
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
