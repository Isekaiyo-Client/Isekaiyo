package net.isekaiyo.client.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import net.isekaiyo.client.core.keybinds.KeybindManager;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class KeybindAndProfileTest {

    @TempDir
    Path dir;

    // ---- keybinds ------------------------------------------------------------

    @Test
    void bindingUnbindingAndDescription() {
        var km = new KeybindManager();
        km.bind("module.a", "Action A", 'R');
        assertEquals("R", km.bindingFor("module.a").describeKey());

        km.unbind("module.a");
        assertTrue(km.bindingFor("module.a").isUnbound());
        assertEquals("unbound", km.bindingFor("module.a").describeKey());

        km.bind("module.b", "Mouse action", KeybindManager.MOUSE_BUTTON_1);
        assertEquals("Mouse 1", km.bindingFor("module.b").describeKey());
    }

    @Test
    void conflictsAreDetectedButNotForbidden() {
        var km = new KeybindManager();
        km.bind("sprint", "Sprint", 'R');
        km.bind("zoom", "Zoom", 'R');
        km.bind("other", "Other", 'F');
        km.bind("free", "Free", KeybindManager.UNBOUND);

        var conflicts = km.conflicts();
        assertEquals(1, conflicts.size(), "only the shared key conflicts");
        String desc = conflicts.get(0).describe();
        assertTrue(desc.contains("sprint") && desc.contains("zoom"));
        assertFalse(desc.contains("free"), "unbound never conflicts");
    }

    @Test
    void actionForKeyResolvesPressesOnly() {
        var km = new KeybindManager();
        km.bind("zoom", "Zoom", 'Z');
        assertEquals("zoom", km.actionForKey('Z', true).actionId());
        assertEquals(null, km.actionForKey('Z', false));
        assertEquals(null, km.actionForKey('X', true));
    }

    // ---- profiles --------------------------------------------------------------

    @Test
    void profileSaveAndSwitchRoundTrip() throws Exception {
        // Real ConfigManager against a temp dir; a real ModuleManager with a
        // no-op sink (profiles drive enable state through setEnabled).
        var config = new net.isekaiyo.client.core.config.ConfigManager(dir);
        var adapter = new ModuleManagerTest.FakeAdapter();
        var modules = new net.isekaiyo.client.core.modules.ModuleManager(
                adapter, new ModuleManagerTest.RecordingSink());
        modules.register(new ModuleManagerTest.NoopModule(
                "m1", net.isekaiyo.client.core.modules.ModuleCategory.HUD,
                new Capability[0], new String[0]));
        modules.register(new ModuleManagerTest.NoopModule(
                "m2", net.isekaiyo.client.core.modules.ModuleCategory.MISC,
                new Capability[0], new String[0]));

        // State: m1 on, m2 off → snapshot as profile "pvp".
        modules.setEnabled("m1", true);
        var profiles = new net.isekaiyo.client.core.profiles.ProfileManager(config, modules);
        profiles.saveCurrentAs("pvp");

        // Change live state away from it, then switch back.
        modules.setEnabled("m1", false);
        modules.setEnabled("m2", true);
        assertTrue(profiles.switchTo("pvp"));
        assertTrue(modules.byId("m1").isEnabled(), "profile restores m1");
        assertFalse(modules.byId("m2").isEnabled(), "profile disables m2");
        assertTrue(profiles.list().contains("pvp"));
    }

    @Test
    void unknownProfileIsRefusedNotCrashy() {
        var config = new net.isekaiyo.client.core.config.ConfigManager(dir);
        var modules = new net.isekaiyo.client.core.modules.ModuleManager(
                new ModuleManagerTest.FakeAdapter(), new ModuleManagerTest.RecordingSink());
        var profiles = new net.isekaiyo.client.core.profiles.ProfileManager(config, modules);
        assertFalse(profiles.switchTo("does-not-exist"));
    }
}
