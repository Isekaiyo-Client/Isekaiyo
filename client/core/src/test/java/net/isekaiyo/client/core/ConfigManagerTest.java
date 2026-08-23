package net.isekaiyo.client.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonObject;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import net.isekaiyo.client.core.config.ConfigManager;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class ConfigManagerTest {

    @TempDir
    Path dir;

    @Test
    void freshInstallYieldsEmptyDocument() {
        var cm = new ConfigManager(dir);
        var doc = cm.load("modules.json");
        assertEquals(ConfigManager.LoadSource.FRESH, doc.source());
        assertNotNull(doc.json());
    }

    @Test
    void existingValidConfigLoads() throws IOException {
        Files.writeString(dir.resolve("client.json"), "{\"theme\":\"midnight\"}");
        var doc = new ConfigManager(dir).load("client.json");
        assertEquals(ConfigManager.LoadSource.OK, doc.source());
        assertEquals("midnight", doc.json().get("theme").getAsString());
    }

    @Test
    void corruptConfigIsBackedUpNotDeleted() throws IOException {
        Path file = dir.resolve("hud.json");
        Files.writeString(file, "{ this is not json");

        var doc = new ConfigManager(dir).load("hud.json");
        assertEquals(ConfigManager.LoadSource.CORRUPT, doc.source());
        assertNotNull(doc.corruptBackupPath());
        assertTrue(Files.exists(doc.corruptBackupPath()),
                "the broken file is preserved for the user (spec §57)");
        assertFalse_(Files.exists(file), "original moved aside so defaults load");
    }

    private static void assertFalse_(boolean b, String msg) {
        org.junit.jupiter.api.Assertions.assertFalse(b, msg);
    }

    @Test
    void unknownAndMissingFieldsAreTolerated() {
        JsonObject withJunk = new JsonObject();
        withJunk.addProperty("known", "x");
        withJunk.addProperty("unknown_future_field", 42); // forward-compat
        var cm = new ConfigManager(dir);
        cm.save("t.json", withJunk);

        var back = cm.load("t.json").json();
        assertEquals("x", back.get("known").getAsString());
        assertTrue(back.has("config_version"), "version stamped on save");
    }

    @Test
    void saveIsAtomicShapeTmpThenFinal() {
        var cm = new ConfigManager(dir);
        JsonObject o = ConfigManager.object();
        o.addProperty("a", 1);
        cm.save("a.json", o);
        // After a successful save no .tmp residue remains.
        try (var s = dirList()) {
            assertTrue(s.noneMatch(n -> n.endsWith(".tmp")));
        }
    }

    private java.util.stream.Stream<String> dirList() {
        try {
            return Files.list(dir).map(p -> p.getFileName().toString());
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }
}
