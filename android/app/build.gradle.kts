plugins {
    id("com.android.application")
}

android {
    namespace = "log.session"
    compileSdk = 36

    defaultConfig {
        applicationId = "log.session"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
}
