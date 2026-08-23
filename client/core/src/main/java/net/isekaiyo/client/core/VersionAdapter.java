package net.isekaiyo.client.core;

import net.isekaiyo.client.core.hud.HudRenderer;

/**
 * The seam between the version-agnostic client and a specific Minecraft
 * version + loader (spec §5). This is the ONLY place game internals may be
 * touched: common code must never branch on {@code minecraftVersion == …};
 * it asks the adapter instead.
 *
 * <p>Implementations live in version-specific projects (e.g. the Fabric
 * modern adapter). Every method documents its thread contract — adapters are
 * responsible for marshalling to the right thread (spec §43).</p>
 */
public interface VersionAdapter {
    /** Static identity info; safe from any thread. */
    AdapterInfo info();

    /**
     * Capabilities THIS adapter genuinely provides. Must be honest: a
     * capability listed here that throws is a bug in the adapter, not an
     * expected condition.
     */
    Capability[] capabilities();

    boolean hasCapability(Capability capability);

    /** Platform config directory for {@code config/isekaiyo/}. */
    java.nio.file.Path configDirectory();

    // ---- game state -------------------------------------------------------
    // All accessors below are callable ONLY on the game/render thread and may
    // return null when the corresponding state does not exist (no world, no
    // player, main menu, …). Common code null-checks; adapters never throw
    // for "absent".

    /** Display name of the local player, or null when not in a world. */
    String playerName();

    /** X/Y/Z of the player, or null when not in a world. Order: [x, y, z]. */
    double[] playerPosition();

    /** Current dimension id, e.g. {@code "overworld"}, or null. */
    String dimensionId();

    /** Name of the connected server, or "Singleplayer", or null. */
    String currentServer();

    /** Current FPS as measured by the game, or -1 when unknown. */
    int currentFps();

    /** Send a chat message client-side; no-op with a log when CHAT is absent. */
    void sendChatMessage(String message);

    /**
     * Schedule {@code task} on the game thread. The primary threading
     * boundary (spec §43): common code must use this for any game-state
     * mutation from other threads.
     */
    void runOnGameThread(Runnable task);

    /**
     * Acquire a HUD renderer for the current frame. Only valid during a
     * {@code HudRenderEvent} dispatch; adapters may return null outside it.
     */
    HudRenderer hudRenderer();

    /**
     * Register adapter-level input forwarding: the adapter translates raw
     * game input into {@code KeyInputEvent}s on the bus. Called once during
     * bootstrap by the client core.
     */
    void hookInput(EventListenerBridge bridge);

    /** Adapter-provided bridge for forwarding raw input to the event bus. */
    interface EventListenerBridge {
        void onKey(int keyCode, int scanCode, int action, int modifiers);

        void onMouseButton(int button, int action, int modifiers);
    }
}
