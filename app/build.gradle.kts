plugins {
    alias(libs.plugins.android.application)
}

android {
    // Was package="com.ridgelineapps.resdicegame" in the manifest. AGP 8 moved it here.
    namespace = "com.ridgelineapps.resdicegame"
    compileSdk = 36

    defaultConfig {
        // Islewright: Roll's own id since going public (2.0.0). Upstream and the 1.x and
        // 2.0 release candidates were com.ridgelineapps.resdicegame: a different app to Android.
        applicationId = "io.github.islewright_roll.islewright"
        // Upstream shipped minSdk 7. Modern AGP will not build that, and nothing in this
        // app needs it: it is plain Canvas drawing on framework views, no AndroidX.
        minSdk = 21
        targetSdk = 36
        // 1.15 (versionCode 16) was the upstream baseline this fork started from.
        versionCode = 33
        versionName = "2.0.0"
    }

    // The signing key is supplied through the environment, never through a file in the
    // repo, so no password or keystore path is ever committed. See README.
    //
    // With no keystore set, the release build is simply unsigned, so a clean checkout
    // still configures and builds.
    val keystorePath: String? = System.getenv("ANDROID_KEYSTORE")

    signingConfigs {
        if (keystorePath != null) {
            create("release") {
                storeFile = file(keystorePath)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS") ?: "islewright"
                // PKCS12 keystores use one password for the store and the key.
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
                    ?: System.getenv("ANDROID_KEYSTORE_PASSWORD")

                // v3 carries a signing certificate lineage, which is the only way to
                // rotate the key later without every install needing an
                // uninstall. Worth having from the first build, since a lineage can
                // only ever extend one that already exists.
                enableV1Signing = true
                enableV2Signing = true
                enableV3Signing = true
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            if (keystorePath != null) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // The soundtrack in res/raw is opened by file descriptor (MediaPlayer), which needs the
    // files stored uncompressed in the APK. aapt already stores the common audio extensions;
    // listing them keeps FLAC (which it does not) and any later change from breaking playback.
    androidResources {
        noCompress += listOf("mp3", "ogg", "flac", "wav", "m4a")
    }

    buildFeatures {
        aidl = false
        buildConfig = false
        shaders = false
    }

    // Local JVM tests of the rules (./gradlew :app:testDebugUnitTest). They run on a
    // developer's machine only, never in a workflow. The android stubs return defaults
    // rather than throwing, so Game's use of SystemClock does not stop a rules test.
    testOptions {
        unitTests {
            isReturnDefaultValues = true
        }
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    // The real org.json, ahead of the android stubs on the test classpath
    testImplementation("org.json:json:20240303")
}
