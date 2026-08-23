// :fabric-modern — the ONLY project in this repository allowed to import
// Minecraft, Fabric API, or mappings. Everything it provides to the rest of
// the client flows through net.isekaiyo.client.core.VersionAdapter.

plugins {
    id("fabric-loom") version "1.9.+"
}

dependencies {
    minecraft("com.mojang:minecraft:${property("minecraft_version")}")
    mappings("net.fabricmc:yarn:${property("yarn_mappings")}:v2")
    modImplementation("net.fabricmc:fabric-loader:${property("loader_version")}")
    modImplementation("net.fabricmc.fabric-api:fabric-api:${property("fabric_version")}")

    implementation(project(":core"))
}
