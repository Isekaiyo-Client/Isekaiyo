package net.isekaiyo.client.core.events;

/**
 * The concrete event set. Deliberately SMALL (spec §10): events model game
 * moments, not method calls. New events are added here, never invented
 * ad-hoc by modules.
 */
public final class Events {

    private Events() {}

    // ---- buses (created once at bootstrap) ---------------------------------

    /** Every client tick while a world is loaded. */
    public static final EventBus<ClientTickEvent> CLIENT_TICK =
            new EventBus<>("ClientTickEvent");

    /** HUD overlay rendering — the only place HUD elements may draw. */
    public static final EventBus<HudRenderEvent> HUD_RENDER =
            new EventBus<>("HudRenderEvent");

    /** Raw keyboard input forwarded by the adapter. */
    public static final EventBus<KeyInputEvent> KEY_INPUT =
            new EventBus<>("KeyInputEvent");

    /** Raw mouse input forwarded by the adapter. */
    public static final EventBus<MouseInputEvent> MOUSE_INPUT =
            new EventBus<>("MouseInputEvent");

    /** The player joined a world/server. */
    public static final EventBus<WorldJoinEvent> WORLD_JOIN =
            new EventBus<>("WorldJoinEvent");

    /** The player left the world/server (also before shutdown). */
    public static final EventBus<WorldLeaveEvent> WORLD_LEAVE =
            new EventBus<>("WorldLeaveEvent");

    /** Fired exactly once during {@code IsekaiyoClient#shutdown()}. */
    public static final EventBus<ClientShutdownEvent> CLIENT_SHUTDOWN =
            new EventBus<>("ClientShutdownEvent");

    /** Remove every subscription owned by {@code owner} across all buses. */
    public static void unsubscribeOwner(String owner) {
        CLIENT_TICK.unsubscribeOwner(owner);
        HUD_RENDER.unsubscribeOwner(owner);
        KEY_INPUT.unsubscribeOwner(owner);
        MOUSE_INPUT.unsubscribeOwner(owner);
        WORLD_JOIN.unsubscribeOwner(owner);
        WORLD_LEAVE.unsubscribeOwner(owner);
        CLIENT_SHUTDOWN.unsubscribeOwner(owner);
    }

    // ---- payloads -----------------------------------------------------------

    /** Marker for parameterless moments; payload records carry real data. */
    public interface Event {}

    public record ClientTickEvent(int tickCount) implements Event {}

    /**
     * Carries the frame's drawing surface. Modules draw ONLY through it —
     * never through Minecraft directly.
     */
    public record HudRenderEvent(net.isekaiyo.client.core.hud.HudRenderer renderer,
                                 int screenWidth, int screenHeight) implements Event {}

    /** action: 0=release 1=press 2=repeat (GLFW convention). */
    public record KeyInputEvent(int keyCode, int scanCode, int action, int modifiers)
            implements Event {
        public boolean isPress() {
            return action == 1;
        }
    }

    public record MouseInputEvent(int button, int action, int modifiers) implements Event {
        public boolean isPress() {
            return action == 1;
        }
    }

    public record WorldJoinEvent(String dimensionId, String serverName) implements Event {}

    public record WorldLeaveEvent() implements Event {}

    public record ClientShutdownEvent() implements Event {}
}
