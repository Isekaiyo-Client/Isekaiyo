// Isekaiyo client build root.
//
// Two products share this repository but never mix:
//   apps/launcher*   — Rust/Tauri launcher (cargo + pnpm)
//   client/*         — JVM client that runs INSIDE Minecraft (Gradle)
//
// The client is deliberately split:
//   :core          — pure Java, ZERO Minecraft imports. Everything here is
//                    unit-testable with plain JUnit on any machine.
//   :fabric-modern — the Fabric adapter + entrypoint for modern versions.
//                    The ONLY project allowed to import Minecraft/Fabric.
//
// Version adapters for other targets (:fabric-legacy, :forge-*, …) are added
// as sibling projects implementing the same VersionAdapter contract.

pluginManagement {
    repositories {
        maven("https://maven.fabricmc.net/") {
            name = "Fabric"
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

rootProject.name = "isekaiyo-client"

include(":core")
include(":fabric-modern")
