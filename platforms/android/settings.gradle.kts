pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(org.gradle.api.initialization.resolve.RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "RadRootsAndroid"

include(":app")
include(":radrootsAndroidSecurity")

project(":radrootsAndroidSecurity").projectDir = file("../../native/bridges/android/security/kotlin/RadRootsAndroidSecurity")
