// MarioRS Android App 模块构建配置

plugins {
    id("com.android.application")
}

android {
    namespace = "com.mariogame"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.mariogame.mario"
        minSdk = 21
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.1"

        ndk {
            // 支持的 ABI 架构
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    // 指定 jniLibs 目录 (由 cargo-ndk 生成的 .so 文件)
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    // 禁用 lint 检查以加快构建
    lint {
        abortOnError = false
    }
}

dependencies {
    // 纯 Native Activity 应用，无需 Java/Kotlin 依赖
}
