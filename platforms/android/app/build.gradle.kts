plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val rustBuildScript = file("../Scripts/build_rust_android.sh")
val rustJniLibsDir = file("../../../target/android/jniLibs")
val rustInputs = files(
    "../../../Cargo.toml",
    "../../../Cargo.lock",
    rustBuildScript,
    fileTree("../../../crates/core"),
    fileTree("../../../crates/android"),
)

android {
    namespace = "org.radroots.app.android"
    compileSdk = 34
    ndkVersion = "26.1.10909125"

    defaultConfig {
        applicationId = "org.radroots.app.android"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    buildTypes {
        debug {}
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir(rustJniLibsDir)
        }
    }
}

val buildRustDebug = tasks.register("buildRustDebug", org.gradle.api.tasks.Exec::class) {
    workingDir = rootDir
    commandLine("bash", rustBuildScript.absolutePath, "Debug")
    inputs.files(rustInputs)
    outputs.dir(rustJniLibsDir)
}

val buildRustRelease = tasks.register("buildRustRelease", org.gradle.api.tasks.Exec::class) {
    workingDir = rootDir
    commandLine("bash", rustBuildScript.absolutePath, "Release")
    inputs.files(rustInputs)
    outputs.dir(rustJniLibsDir)
}

afterEvaluate {
    tasks.named("preDebugBuild").configure {
        dependsOn(buildRustDebug)
    }
    tasks.named("preReleaseBuild").configure {
        dependsOn(buildRustRelease)
    }
}

dependencies {
    implementation("androidx.games:games-activity:2.0.2")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core-ktx:1.13.1")
    implementation(project(":radrootsAndroidSecurity"))
}
