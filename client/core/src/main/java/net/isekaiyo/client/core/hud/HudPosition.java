package net.isekaiyo.client.core.hud;

/** Screen anchors for HUD elements (spec §18). */
public enum HudPosition {
    TOP_LEFT,
    TOP_CENTER,
    TOP_RIGHT,
    BOTTOM_LEFT,
    BOTTOM_CENTER,
    BOTTOM_RIGHT;

    public boolean isBottom() {
        return this == BOTTOM_LEFT || this == BOTTOM_CENTER || this == BOTTOM_RIGHT;
    }
}
