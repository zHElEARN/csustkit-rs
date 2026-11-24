plugins {
    kotlin("jvm") version "2.2.20"
    application
}

kotlin {
    jvmToolchain(17)
}

application {
    mainClass.set("com.zhelearn.csustkit.example.MainKt")
}

dependencies {
    implementation("net.java.dev.jna:jna:5.18.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
}

val rustRoot = layout.projectDirectory.dir("../..")
val cargoManifest = rustRoot.file("Cargo.toml")
val cargoLock = rustRoot.file("Cargo.lock")
val cargoTarget = layout.buildDirectory.dir("cargo")
val libraryName = System.mapLibraryName("csustkit")
val nativeLibrary = cargoTarget.map { it.file("debug/$libraryName") }
val kotlinBindings = layout.buildDirectory.dir("generated/kotlin")

val buildRust by tasks.registering(Exec::class) {
    workingDir = rustRoot.asFile
    inputs.files(cargoManifest, cargoLock, fileTree(rustRoot.dir("src")))
    inputs.property("hostArchitecture", System.getProperty("os.arch"))
    outputs.file(nativeLibrary)

    environment("CARGO_TARGET_DIR", cargoTarget.get().asFile)
    commandLine("cargo", "rustc", "--locked", "--features", "uniffi", "--lib", "--crate-type", "cdylib")
}

val generateKotlinBindings by tasks.registering(Exec::class) {
    dependsOn(buildRust)
    workingDir = rustRoot.asFile
    inputs.files(cargoManifest, cargoLock, rustRoot.file("uniffi.toml"), rustRoot.file("tools/csustkit-bindgen.rs"))
    inputs.file(nativeLibrary)
    outputs.dir(kotlinBindings)

    environment("CARGO_TARGET_DIR", cargoTarget.get().asFile)
    commandLine(
        "cargo", "run", "--locked", "--features", "uniffi-bindgen", "--bin", "csustkit-bindgen", "--",
        "generate", "--language", "kotlin", "--no-format", "--metadata-no-deps",
        "--out-dir", kotlinBindings.get().asFile.absolutePath, nativeLibrary.get().asFile.absolutePath,
    )
    doFirst {
        delete(kotlinBindings)
    }
}

kotlin.sourceSets.named("main") {
    kotlin.srcDir(generateKotlinBindings)
}

tasks.processResources {
    from(buildRust)
}

tasks.named<JavaExec>("run") {
    workingDir = rustRoot.asFile
    standardInput = System.`in`
}
