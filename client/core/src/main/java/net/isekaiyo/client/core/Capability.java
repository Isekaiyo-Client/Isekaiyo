package net.isekaiyo.client.core;

/**
 * What a version adapter can actually do (spec §6). Modules declare the
 * capabilities they require; an adapter that cannot provide them makes the
 * module <em>Unavailable</em> — never a crash on old versions.
 */
public enum Capability {
    HUD,
    KEYBINDS,
    SCREEN_RENDERING,
    WORLD_RENDERING,
    CHAT,
    INPUT,
    SOUND,
    /** Access to the local player object (absent on some odd setups). */
    PLAYER_STATE,
    /** Current dimension/world info. */
    WORLD_STATE;

    /** Convenience for module declarations. */
    public static Capability[] of(Capability... capabilities) {
        return capabilities;
    }
}
