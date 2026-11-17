import org.jetbrains.kotlin.gradle.dsl.JvmTarget

// Sem versão aqui: dentro do app (src-tauri/gen/android) valem as versões do
// projeto; sozinho, as do settings.gradle.kts deste plugin.
plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "app.renovador.bg"
    compileSdk = 36

    defaultConfig {
        // java.time e WorkManager sem desugaring.
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.work:work-runtime-ktx:2.10.0")
    implementation(project(":tauri-android"))
    testImplementation("junit:junit:4.13.2")
    // org.json real nos testes JVM (no android.jar ele é só um stub)
    testImplementation("org.json:json:20240303")
}
