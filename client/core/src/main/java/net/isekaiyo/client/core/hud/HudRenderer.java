package net.isekaiyo.client.core.hud;

import net.isekaiyo.client.core.theme.ThemeTokens;

/**
 * The drawing surface handed to {@code HudRenderEvent} (spec §19). Adapters
 * implement this over Minecraft's renderer; elements never touch Minecraft
 * rendering directly. All coordinates are GUI-scaled pixels.
 */
public interface HudRenderer {
    int screenWidth();

    int screenHeight();

    /** Draw a string with the theme's text color at scale 1. Returns width. */
    int drawText(String text, int x, int y, int argb);

    /** Draw a string with an explicit drop shadow. Returns width. */
    int drawTextWithShadow(String text, int x, int y, int argb);

    /** Measure text width without drawing. */
    int textWidth(String text);

    /** Fill a rectangle. */
    void fill(int x1, int y1, int x2, int y2, int argb);

    /** Convenience: translucent panel using theme tokens. */
    default void panel(ThemeTokens tokens, int x, int y, int width, int height) {
        fill(x, y, x + width, y + height, tokens.surfaceArgb());
        // 1px border in border color.
        fill(x, y, x + width, y + 1, tokens.borderArgb());
        fill(x, y + height - 1, x + width, y + height, tokens.borderArgb());
        fill(x, y, x + 1, y + height, tokens.borderArgb());
        fill(x + width - 1, y, x + width, y + height, tokens.borderArgb());
    }
}
