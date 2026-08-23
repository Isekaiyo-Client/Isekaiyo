package net.isekaiyo.client.core;

import java.util.Objects;

/**
 * Identity of the running game + client, for the debug view (spec §54).
 * Never contains credentials or anything sensitive.
 */
public record AdapterInfo(
        String minecraftVersion,
        String loaderName,
        String loaderVersion,
        String isekaiyoVersion,
        String javaVersion
) {
    public AdapterInfo {
        Objects.requireNonNull(minecraftVersion, "minecraftVersion");
        Objects.requireNonNull(loaderName, "loaderName");
        // Optional fields may be "unknown" but never null.
        loaderVersion = loaderVersion == null ? "unknown" : loaderVersion;
        isekaiyoVersion = isekaiyoVersion == null ? "unknown" : isekaiyoVersion;
        javaVersion = javaVersion == null ? "unknown" : javaVersion;
    }

    /** Multi-line debug dump; safe to show on screen. */
    public String debugText() {
        return "Isekaiyo " + isekaiyoVersion + "\n"
                + "Minecraft " + minecraftVersion + "\n"
                + "Loader: " + loaderName + " " + loaderVersion + "\n"
                + "Java " + javaVersion;
    }
}
