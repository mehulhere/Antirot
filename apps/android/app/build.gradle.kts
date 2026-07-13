plugins {
    id("com.android.application")
}

val googleWebClientId = providers.environmentVariable("ANTIROT_GOOGLE_WEB_CLIENT_ID")
    .orElse("")
    .get()
    .replace("\\", "\\\\")
    .replace("\"", "\\\"")
val releaseKeystorePath = providers.environmentVariable("ANDROID_SIGNING_KEYSTORE_PATH").orElse("").get()
val releaseKeyAlias = providers.environmentVariable("ANDROID_SIGNING_KEY_ALIAS").orElse("").get()
val releaseStorePassword = providers.environmentVariable("ANDROID_SIGNING_STORE_PASSWORD").orElse("").get()
val releaseKeyPassword = providers.environmentVariable("ANDROID_SIGNING_KEY_PASSWORD").orElse("").get()

android {
    namespace = "com.mehulhere.antirot"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.mehulhere.antirot"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        buildConfigField("String", "GOOGLE_WEB_CLIENT_ID", "\"$googleWebClientId\"")
    }

    buildFeatures {
        buildConfig = true
    }

    signingConfigs {
        create("release") {
            if (releaseKeystorePath.isNotEmpty()) {
                storeFile = file(releaseKeystorePath)
                storePassword = releaseStorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
        }
    }
}

dependencies {
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    implementation("androidx.work:work-runtime:2.10.5")
    implementation("com.google.android.gms:play-services-auth:21.4.0")
}

dependencyLocking {
    lockAllConfigurations()
}
