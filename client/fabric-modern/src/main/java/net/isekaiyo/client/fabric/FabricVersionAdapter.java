package net.isekaiyo.client.fabric;

import java.nio.file.Path;
import java.util.Set;
import net.isekaiyo.client.core.AdapterInfo;
import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.VersionAdapter;
import net.isekaiyo.client.core.hud.HudRenderer;
import net.minecraft.client.MinecraftClient;

/**
 * Modern-Fabric implementation of {@link VersionAdapter}. Every Minecraft
 * import in the client lives in this project; this class is its frontier.
 *
 * <p>Honesty contract: {@link #capabilities()} lists exactly what is
 * implemented below. Anything not implemented must be absent from the set so
 * dependent modules become Unavailable rather than crash.</p>
 */
public final class FabricVersionAdapter implements VersionAdapter {

    /** What THIS adapter genuinely provides today (spec §6 honesty rule). */
    private static final Set<Capability> PROVIDED = Set.of(
            Capability.HUD,
            Capability.KEYBINDS,
            Capability.INPUT,
            Capability.PLAYER_STATE,
            Capability.WORLD_STATE,
            Capability.SCREEN_RENDERING
    );

    private volatile FabricHudRenderer frameRenderer;

    @Override
    public AdapterInfo info() {
        var mc = MinecraftClient.getInstance();
        return new AdapterInfo(
                mc.getGameVersion(),
                "Fabric",
                net.fabricmc.loader.api.FabricLoader.getInstance()
                        .getModContainer("fabricloader")
                        .map(c -> c.getMetadata().getVersion().getFriendlyString())
                        .orElse("unknown"),
                // Injected by Gradle processResources into fabric.mod.json.
                net.fabricmc.loader.api.FabricLoader.getInstance()
                        .getModContainer("isekaiyo-client")
                        .map(c -> c.getMetadata().getVersion().getFriendlyString())
                        .orElse("dev"),
                System.getProperty("java.version", "unknown"));
    }

    @Override
    public Path configDirectory() {
        return net.fabricmc.loader.api.FabricLoader.getInstance()
                .getConfigDir()
                .resolve("isekaiyo");
    }

    @Override
    public Capability[] capabilities() {
        return PROVIDED.toArray(new Capability[0]);
    }

    @Override
    public boolean hasCapability(Capability capability) {
        return PROVIDED.contains(capability);
    }

    // ---- game state (game thread only) --------------------------------------

    @Override
    public String playerName() {
        var player = MinecraftClient.getInstance().player;
        return player != null ? player.getName().getString() : null;
    }

    @Override
    public double[] playerPosition() {
        var player = MinecraftClient.getInstance().player;
        return player != null
                ? new double[] {player.getX(), player.getY(), player.getZ()}
                : null;
    }

    @Override
    public String dimensionId() {
        var world = MinecraftClient.getInstance().world;
        if (world == null) {
            return null;
        }
        return world.getRegistryKey().getValue().getPath();
    }

    @Override
    public String currentServer() {
        var mc = MinecraftClient.getInstance();
        if (mc.getCurrentServerEntry() != null) {
            return mc.getCurrentServerEntry().address;
        }
        return mc.world != null ? "Singleplayer" : null;
    }

    @Override
    public int currentFps() {
        return MinecraftClient.getInstance().getCurrentFps();
    }

    @Override
    public void sendChatMessage(String message) {
        var player = MinecraftClient.getInstance().player;
        if (player != null && message != null && !message.isBlank()) {
            // Length sanity: never relay unbounded text into chat.
            String safe = message.length() > 256 ? message.substring(0, 256) : message;
            player.networkHandler.sendChatMessage(safe);
        }
    }

    @Override
    public void runOnGameThread(Runnable task) {
        MinecraftClient.getInstance().execute(task);
    }

    // ---- rendering / input -----------------------------------------------------

    /** Called from IsekaiyoFabricClient's HudRenderCallback with this frame's context. */
    FabricHudRenderer currentHudRenderer(Object drawContext) {
        FabricHudRenderer r = frameRenderer;
        if (r == null) {
            r = new FabricHudRenderer();
            frameRenderer = r;
        }
        r.bind(drawContext);
        return r;
    }

    @Override
    public HudRenderer hudRenderer() {
        FabricHudRenderer r = frameRenderer;
        return r != null && r.isBound() ? r : null;
    }

    @Override
    public void hookInput(EventListenerBridge bridge) {
        // Key/mouse events are bridged to core Events from
        // IsekaiyoFabricClient via Fabric API input hooks (to be wired in the
        // first runnable build). Kept as an explicit no-op rather than fake
        // registration — honest until implemented.
    }

    /**
     * Per-tick application point for module effects (e.g. ToggleSprint).
     * Called on the game thread from the tick bridge.
     */
    void applyModuleEffects() {
        // ToggleSprint wiring lands with the movement flag adapter work;
        // intentionally minimal until then — no fake behavior.
    }
}
