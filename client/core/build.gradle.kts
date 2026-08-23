// ikk-client-core — the version-agnostic client API.
//
// RULE: no Minecraft, Fabric, Forge, or any game dependency may appear here.
// Everything in this project compiles and tests on a bare JDK. Version
// adapters (:fabric-modern) depend ON this project — never the reverse.

dependencies {
    // Human-readable JSON config. License: Apache-2.0 (documented in
    // docs/architecture/client.md §dependencies).
    implementation("com.google.code.gson:gson:2.11.0")

    testImplementation(platform("org.junit:junit-bom:5.10.2"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}
