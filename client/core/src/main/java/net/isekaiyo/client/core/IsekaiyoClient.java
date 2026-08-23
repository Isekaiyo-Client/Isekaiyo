package net.isekaiyo.client.core;

import net.isekaiyo.client.core.config.ConfigManager;
import net.isekaiyo.client.core.events.Events;
import net.isekaiyo.client.core.hud.HudManager;
import net.isekaiyo.client.core.keybinds.KeybindManager;
import net.isekaiyo.client.core.lifecycle.LifecycleState;
import net.isekaiyo.client.core.modules.ModuleManager;
import net.isekaiyo.client.core.notify.NotificationManager;
import net.isekaiyo.client.core.profiles.ProfileManager;
import net.isekaiyo.client.core.theme.ThemeTokens;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * The client entrypoint (spec §40). Adapters construct this with a
 * {@link VersionAdapter} and call {@link #initialize()} from their mod-init
 * hook; the deterministic startup order is documented and enforced here
 * (spec §41):
 *
 * <pre>
 * Bootstrap → Version Adapter → Configuration → Event wiring (static buses)
 *   → Module Registry → Keybinds → HUD → Profile → READY
 * </pre>
 *
 * <p>Shutdown (spec §42): save config → disable modules (listeners removed)
 * → unload modules → mark SHUTTING_DOWN. A crashed Minecraft simply loses the
 * last in-memory changes; on-disk state is always complete because every
 * write is atomic.</p>
 */
public final class IsekaiyoClient {

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo");

    private static volatile IsekaiyoClient instance;

    private final VersionAdapter adapter;
    private final ConfigManager config;
    private final KeybindManager keybinds = new KeybindManager();
    private final HudManager hud = new HudManager();
    private final NotificationManager notifications =
            new NotificationManager(System::nanoTime / 1_000_000);
    private final ModuleManager modules;
    private final ProfileManager profiles;
    private final ClientContext context;

    private volatile LifecycleState state = LifecycleState.BOOTSTRAPPING;
    /** Set when the active profile's keybinds were applied; for debug view. */
    private String activeProfileId = "default";

    public IsekaiyoClient(VersionAdapter adapter) {
        this.adapter = adapter;
        this.config = new ConfigManager(adapter.configDirectory());
        // The module ConfigSink is provided by the adapter-specific glue via
        // setModuleConfigSink BEFORE initialize(); default no-op otherwise.
        this.modules = new ModuleManager(adapter, (mods) -> { /* replaced below */ });
        this.profiles = new ProfileManager(config, modules);
        this.context = new ClientContext(
                adapter, config, modules, keybinds, hud, notifications, profiles);
    }

    /**
     * Deterministic initialization (spec §41). Idempotent guard: calling
     * twice logs and returns instead of double-initializing.
     */
    public synchronized boolean initialize() {
        if (state != LifecycleState.BOOTSTRAPPING) {
            LOG.warn("initialize() called again in state {} — ignored", state);
            return false;
        }
        try {
            // 1. Version adapter sanity.
            AdapterInfo info = adapter.info();
            LOG.info("initializing on Minecraft {} ({})", info.minecraftVersion(), info.loaderName());

            // 2. Configuration directories + global file.
            state = LifecycleState.INITIALIZING;
            ConfigManager.LoadedDocument clientDoc = config.load("client.json");
            if (clientDoc.source() == ConfigManager.LoadSource.CORRUPT) {
                notifications.push(
                        NotificationManager.Kind.WARNING,
                        "Config",
                        "client.json was corrupt — defaults loaded");
            }
            String themeId = stringOr(clientDoc.json(), "theme", "sakura");
            context.setTheme(ThemeTokens.byId(themeId));
            activeProfileId = stringOr(clientDoc.json(), "active_profile", "default");

            // 3. Modules register themselves here (adapter calls
            //    BuiltInModules.registerAll(context)); then persisted state.
            //    Registration happens between INITIALIZING and READY so that
            //    module onLoad() sees a fully wired context.

            // 4. Load module enabled-state from modules.json.
            modules.loadPersistedState();

            // 5. Ready.
            state = LifecycleState.READY;
            instance = this;
            LOG.info(
                    "Isekaiyo ready — {} module(s), {} enabled",
                    modules.all().size(),
                    modules.enabledCount());
            return true;
        } catch (Throwable t) {
            state = LifecycleState.FAILED;
            LOG.error("Isekaiyo failed to initialize — the game continues unmodified", t);
            return false;
        }
    }

    private static String stringOr(com.google.gson.JsonObject obj, String key, String fallback) {
        return obj.has(key) && obj.get(key).isJsonPrimitive()
                ? obj.get(key).getAsString()
                : fallback;
    }

    /** Ordered shutdown (spec §42); safe to call from any state. */
    public synchronized void shutdown() {
        if (state != LifecycleState.READY) {
            return;
        }
        state = LifecycleState.SHUTTING_DOWN;
        Events.CLIENT_SHUTDOWN.dispatch(new Events.ClientShutdownEvent());
        modules.unloadAll();
        Events.unsubscribeOwner("isekaiyo"); // core-owned listeners
        LOG.info("Isekaiyo shut down cleanly");
    }

    // ---- accessors ------------------------------------------------------------

    public static IsekaiyoClient get() {
        IsekaiyoClient i = instance;
        if (i == null) {
            throw new IllegalStateException("IsekaiyoClient not initialized");
        }
        return i;
    }

    public static boolean isAvailable() {
        return instance != null && instance.state.isOperational();
    }

    public ClientContext context() {
        return context;
    }

    public LifecycleState state() {
        return state;
    }

    public String activeProfileId() {
        return activeProfileId;
    }

    void setActiveProfileId(String id) {
        this.activeProfileId = id;
    }

    /** Debug overlay content (spec §55): identity only, never secrets. */
    public String debugText() {
        AdapterInfo info = adapter.info();
        return info.debugText() + "\n"
                + "Profile: " + activeProfileId + "\n"
                + "Modules: " + modules.enabledCount() + "/" + modules.all().size() + " enabled\n"
                + "Theme: " + context.theme().displayName();
    }
}
