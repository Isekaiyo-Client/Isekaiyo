package net.isekaiyo.client.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.util.Collection;
import java.util.List;
import net.isekaiyo.client.core.events.Events;
import net.isekaiyo.client.core.modules.Module;
import net.isekaiyo.client.core.modules.ModuleCategory;
import net.isekaiyo.client.core.modules.ModuleManager;
import net.isekaiyo.client.core.settings.Setting;
import net.isekaiyo.client.core.settings.StandardSettings;
import org.junit.jupiter.api.Test;

class ModuleManagerTest {

    /** Minimal adapter: capabilities configurable per test. */
    static final class FakeAdapter implements VersionAdapter {
        final Set<Capability> caps;

        FakeAdapter(Capability... caps) {
            this.caps = java.util.Set.of(caps);
        }

        @Override
        public AdapterInfo info() {
            return new AdapterInfo("1.0", "test", "1", "dev", "21");
        }

        @Override
        public Path configDirectory() {
            return Path.of(System.getProperty("java.io.tmpdir"), "ikk-test-config");
        }

        @Override
        public Capability[] capabilities() {
            return caps.toArray(new Capability[0]);
        }

        @Override
        public boolean hasCapability(Capability c) {
            return caps.contains(c);
        }

        @Override
        public String playerName() {
            return null;
        }

        @Override
        public double[] playerPosition() {
            return null;
        }

        @Override
        public String dimensionId() {
            return null;
        }

        @Override
        public String currentServer() {
            return null;
        }

        @Override
        public int currentFps() {
            return 60;
        }

        @Override
        public void sendChatMessage(String message) {}

        @Override
        public void runOnGameThread(Runnable task) {
            task.run();
        }

        @Override
        public net.isekaiyo.client.core.hud.HudRenderer hudRenderer() {
            return null;
        }

        @Override
        public void hookInput(EventListenerBridge bridge) {}
    }

    /** Recording config sink for persistence assertions. */
    static final class RecordingSink implements ModuleManager.ConfigSink {
        int saves;
        int loads;
        boolean lastAllEnabled;

        @Override
        public void saveModules(Collection<Module> modules) {
            saves++;
            lastAllEnabled = modules.stream().allMatch(m -> !m.isEnabled() || m.isEnabled());
        }

        @Override
        public void loadModules(Collection<Module> modules) {}
    }

    static final class NoopModule extends Module {
        NoopModule(String id, ModuleCategory cat, Capability[] reqs, String[] deps) {
            super(id, id.toUpperCase(), "test module " + id, cat, reqs, deps);
        }

        @Override
        protected void onEnable() {
            Events.CLIENT_TICK.subscribe(id(), e -> {});
        }
    }

    private static RecordingSink sink;

    private ModuleManager manager(FakeAdapter adapter) {
        sink = new RecordingSink();
        return new ModuleManager(adapter, sink);
    }

    @Test
    void registrationLookupAndDuplicateDetection() {
        ModuleManager mm = manager(new FakeAdapter());
        mm.register(new NoopModule("a", ModuleCategory.MISC, new Capability[0], new String[0]));
        assertNotNull(mm.byId("a"));
        assertThrows(IllegalArgumentException.class,
                () -> mm.register(new NoopModule("a", ModuleCategory.MISC,
                        new Capability[0], new String[0])));
    }

    @Test
    void missingCapabilityMakesModuleUnavailableNotCrashy() {
        // Module needs HUD; adapter doesn't provide it.
        ModuleManager mm = manager(new FakeAdapter());
        mm.register(new NoopModule("hud-thing", ModuleCategory.HUD,
                Capability.of(Capability.HUD), new String[0]));
        assertTrue(mm.byId("hud-thing").isUnavailable());
        assertFalse(mm.setEnabled("hud-thing", true));
        assertEquals(0, mm.enabledCount());
    }

    @Test
    void disableRemovesEventListeners() {
        ModuleManager mm = manager(new FakeAdapter());
        mm.register(new NoopModule("m", ModuleCategory.MISC, new Capability[0], new String[0]));
        assertTrue(mm.setEnabled("m", true));
        int before = Events.CLIENT_TICK.listenerCount();
        assertEquals(1, before);
        assertTrue(mm.toggle("m"));
        assertEquals(before - 1, Events.CLIENT_TICK.listenerCount(),
                "disable must tear down the module's subscriptions");
        assertFalse(mm.byId("m").isEnabled());
        assertTrue(sink.saves >= 2, "state persisted on each change");
    }

    @Test
    void dependencyOnUnavailableModuleBlocksRegistrationAvailability() {
        ModuleManager mm = manager(new FakeAdapter());
        mm.register(new NoopModule("hud-core", ModuleCategory.HUD,
                Capability.of(Capability.HUD), new String[0]));
        mm.register(new NoopModule("advanced", ModuleCategory.HUD,
                new Capability[0], new String[] {"hud-core"}));
        assertTrue(mm.byId("advanced").isUnavailable(),
                "dependency unavailable → dependent unavailable");
    }

    @Test
    void searchMatchesMetadataCaseInsensitively() {
        ModuleManager mm = manager(new FakeAdapter());
        mm.register(new NoopModule("fps", ModuleCategory.HUD, new Capability[0], new String[0]));
        mm.register(new NoopModule("zoom", ModuleCategory.RENDER, new Capability[0], new String[0]));
        List<Module> hits = mm.search("FP");
        assertEquals(1, hits.size());
        assertEquals("fps", hits.get(0).id());
        assertEquals(List.of(ModuleCategory.HUD, ModuleCategory.RENDER), mm.presentCategories());
    }

    @Test
    void settingsDiscoveryFindsTypedFields() {
        ModuleManager mm = manager(new FakeAdapter());
        mm.register(new NoopModule("with-settings", ModuleCategory.HUD,
                new Capability[0], new String[0]) {
            final Setting<Boolean> flag = StandardSettings.bool("flag", true);
            final Setting<Integer> level = StandardSettings.integer("level", 3, 1, 10);
        });
        Module m = mm.byId("with-settings");
        assertEquals(2, m.settings().size());
        assertNotNull(m.setting("level"));
        assertTrue(m.setting("level").trySet(7));
        assertFalse(m.setting("level").trySet(99), "out of bounds rejected");
        assertEquals(3, ((StandardSettings.IntSetting) m.setting("level")).get(),
                "invalid set keeps previous value");
    }
}
