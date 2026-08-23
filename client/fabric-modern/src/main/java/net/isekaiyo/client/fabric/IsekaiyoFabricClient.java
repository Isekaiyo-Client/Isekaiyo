package net.isekaiyo.client.fabric;

import net.isekaiyo.client.core.IsekaiyoClient;
import net.isekaiyo.client.core.events.Events;
import net.isekaiyo.client.core.modules.builtin.BuiltInModules;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Fabric entrypoint (spec §40). Responsibilities, in order:
 * 1. construct the client with {@link FabricVersionAdapter}
 * 2. register built-in modules
 * 3. bridge Fabric events → core buses (tick, HUD render)
 * 4. initialize
 *
 * This class contains no game logic — only wiring.
 */
public final class IsekaiyoFabricClient implements ClientModInitializer {

    public static final Logger LOG = LoggerFactory.getLogger("Isekaiyo");

    private IsekaiyoClient client;

    @Override
    public void onInitializeClient() {
        FabricVersionAdapter adapter = new FabricVersionAdapter();
        client = new IsekaiyoClient(adapter);

        // Built-in registration happens before loadPersistedState inside
        // initialize()'s INITIALIZING phase ordering; register here explicitly
        // so module onLoad() sees a wired context.
        BuiltInModules.registerAll(client, client.context());

        // --- event bridging -------------------------------------------------

        // Tick: forward once per client tick.
        ClientTickEvents.END_CLIENT_TICK.register(mc -> {
            if (!IsekaiyoClient.isAvailable()) {
                return;
            }
            Events.CLIENT_TICK.dispatch(
                    new Events.ClientTickEvent((int) (mc.world == null ? -1 : mc.world.getTime())));
            adapter.applyModuleEffects();
        });

        // HUD render: hand the frame's drawing surface straight through.
        HudRenderCallback.EVENT.register((drawContext, tickDelta) -> {
            if (!IsekaiyoClient.isAvailable()) {
                return;
            }
            var mc = net.minecraft.client.MinecraftClient.getInstance();
            int w = mc.getWindow().getScaledWidth();
            int h = mc.getWindow().getScaledHeight();
            var event = new Events.HudRenderEvent(
                    adapter.currentHudRenderer(drawContext), w, h);
            // The single render pass over enabled elements:
            client.context().hud().render(event);
        });

        // Shutdown: ordered teardown.
        Runtime.getRuntime().addShutdownHook(new Thread(client::shutdown, "isekaiyo-shutdown"));

        boolean ok = client.initialize();
        if (ok) {
            LOG.info("Isekaiyo initialized");
        }
        // Failure is non-fatal by design: the game runs unmodified.
    }

    private int[] mcWindow() {
        var mc = net.minecraft.client.MinecraftClient.getInstance();
        var w = mc.getWindow();
        return new int[] {w.getScaledWidth(), w.getScaledHeight()};
    }
}
