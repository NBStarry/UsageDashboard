import java.util.Properties
import java.io.File
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// release 签名:从 gen/android/keystore.properties 读密钥(不入库,见 .gitignore)。
val keystoreProperties = Properties().apply {
    val propFile = rootProject.file("keystore.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    namespace = "app.usagedashboard"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "true"
        applicationId = "app.usagedashboard"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        create("release") {
            keystoreProperties.getProperty("storeFile")?.let {
                storeFile = rootProject.file(it)
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

val androidArtifactStamp: String =
    System.getenv("USAGE_DASHBOARD_ANDROID_ARTIFACT_STAMP")
        ?: LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyyMMdd-HHmmss"))
val androidArtifactRoot: File = rootProject.file("../../../../release-apks")
val androidArtifactDir: File = File(androidArtifactRoot, androidArtifactStamp)
val androidLatestArtifactDir: File = File(androidArtifactRoot, "latest")

val collectAndroidArtifacts = tasks.register("collectAndroidArtifacts") {
    group = "build"
    description = "Copy built APK/AAB artifacts to release-apks/<timestamp> and release-apks/latest."

    doLast {
        val outputRoot = layout.buildDirectory.dir("outputs").get().asFile
        val artifacts = fileTree(outputRoot) {
            include("apk/**/*.apk")
            include("bundle/**/*.aab")
        }.files.sortedWith(compareBy<File> { it.extension }.thenBy { it.name })

        if (artifacts.isEmpty()) {
            logger.lifecycle("No Android APK/AAB artifacts found under ${outputRoot.absolutePath}")
            return@doLast
        }

        androidArtifactDir.mkdirs()
        if (androidLatestArtifactDir.exists()) {
            androidLatestArtifactDir.deleteRecursively()
        }
        androidLatestArtifactDir.mkdirs()

        val manifest = buildString {
            appendLine("UsageDashboard Android artifacts")
            appendLine("Created: ${LocalDateTime.now().format(DateTimeFormatter.ISO_LOCAL_DATE_TIME)}")
            appendLine("Archive: ${androidArtifactDir.absolutePath}")
            appendLine()
            appendLine("Files:")
            for (source in artifacts) {
                val dest = File(androidArtifactDir, source.name)
                val latestDest = File(androidLatestArtifactDir, source.name)
                source.copyTo(dest, overwrite = true)
                source.copyTo(latestDest, overwrite = true)
                appendLine("- ${source.name}")
                appendLine("  source: ${source.absolutePath}")
                appendLine("  size: ${source.length()} bytes")
            }
        }

        File(androidArtifactDir, "README.txt").writeText(manifest)
        File(androidLatestArtifactDir, "README.txt").writeText(manifest)
        File(androidArtifactRoot, "LATEST.txt").writeText(androidArtifactDir.absolutePath + System.lineSeparator())
        logger.lifecycle("Copied Android artifacts to ${androidArtifactDir.absolutePath}")
        logger.lifecycle("Latest Android artifacts are also in ${androidLatestArtifactDir.absolutePath}")
    }
}

tasks.matching {
    (it.name.startsWith("assemble") || it.name.startsWith("bundle") || it.name.startsWith("package")) &&
        (it.name.endsWith("Debug") || it.name.endsWith("Release"))
}.configureEach {
    finalizedBy(collectAndroidArtifacts)
}

apply(from = "tauri.build.gradle.kts")
