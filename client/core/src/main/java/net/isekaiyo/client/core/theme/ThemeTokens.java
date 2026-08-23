package net.isekaiyo.client.core.theme;

/**
 * Centralized color tokens (spec §25). Modules and HUD elements read these;
 * raw hex values never appear outside this package.
 *
 * <p>Colors are ARGB ints (0xAARRGGBB), matching Minecraft conventions.</p>
 */
public record ThemeTokens(
        String id,
        String displayName,
        int backgroundArgb,
        int surfaceArgb,
        int surfaceElevatedArgb,
        int borderArgb,
        int textArgb,
        int textMutedArgb,
        int accentArgb,
        int accentSoftArgb,
        int dangerArgb,
        int warningArgb,
        int successArgb
) {
    /** The Isekaiyo identity: AMOLED black + sakura pink. */
    public static ThemeTokens sakura() {
        return new ThemeTokens(
                "sakura",
                "Isekaiyo Sakura",
                0xFF000000,
                0xE60A0A0C,
                0xF5101014,
                0xFF1D1D22,
                0xFFE8E8EC,
                0xFF8B8B95,
                0xFFFF5C8A,
                0x28FF5C8A,
                0xFFFF6B6B,
                0xFFFFB454,
                0xFF6EE7A0
        );
    }

    /** Calm dark-blue alternative. */
    public static ThemeTokens midnight() {
        return new ThemeTokens(
                "midnight",
                "Midnight",
                0xFF05070D,
                0xE60D1119,
                0xF5121822,
                0xFF20293A,
                0xFFE4E9F2,
                0xFF7F8AA3,
                0xFF6FA8FF,
                0x286FA8FF,
                0xFFFF6B6B,
                0xFFFFB454,
                0xFF6EE7A0
        );
    }

    /** Near-monochrome, maximum readability. */
    public static ThemeTokens minimal() {
        return new ThemeTokens(
                "minimal",
                "Minimal",
                0xFF000000,
                0xD9101010,
                0xEE181818,
                0xFF262626,
                0xFFF2F2F2,
                0xFF909090,
                0xFFDDDDDD,
                0x22DDDDDD,
                0xFFFF6B6B,
                0xFFFFB454,
                0xFF6EE7A0
        );
    }

    public static ThemeTokens byId(String id) {
        return switch (id == null ? "" : id) {
            case "midnight" -> midnight();
            case "minimal" -> minimal();
            default -> sakura();
        };
    }
}
