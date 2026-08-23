package net.isekaiyo.client.core.profiles;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import net.isekaiyo.client.core.config.ConfigManager;
import net.isekaiyo.client.core.modules.Module;
import net.isekaiyo.client.core.modules.ModuleManager;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Client-side profiles (spec §31–§33): named snapshots of
 * enabled-module sets + settings + theme. Profiles are pure client state —
 * never tied to launcher accounts.
 *
 * <p>Switching is hot: disable current set, apply the target's, persist. A
 * profile referencing unknown modules skips those entries and logs (spec §33).
 * The active profile id lives in {@code client.json}.</p>
 */
public final class ProfileManager {

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo/Config");

    private final ConfigManager config;
    private final ModuleManager modules;
    private final Path profilesDir;

    public ProfileManager(ConfigManager config, ModuleManager modules) {
        this.config = config;
        this.modules = modules;
        this.profilesDir = config.root().resolve("profiles");
    }

    /** Persist the CURRENT live state under a profile name. */
    public void saveCurrentAs(String profileId) {
        JsonObject doc = ConfigManager.object();
        JsonArray enabled = new JsonArray();
        for (Module m : modules.all()) {
            if (m.isEnabled()) {
                enabled.add(m.id());
            }
        }
        doc.add("enabled_modules", enabled);
        config.save("profiles/" + sanitize(profileId) + ".json", doc);
    }

    public List<String> list() {
        List<String> names = new ArrayList<>();
        try (var stream = java.nio.file.Files.list(profilesDir)) {
            stream
                    .map(p -> p.getFileName().toString())
                    .filter(n -> n.endsWith(".json"))
                    .map(n -> n.substring(0, n.length() - ".json".length()))
                    .sorted()
                    .forEach(names::add);
        } catch (Exception e) {
            LOG.info("no profiles directory yet — empty list");
        }
        return names;
    }

    /**
     * Switch to {@code profileId}: disable everything not in the profile,
     * enable everything in it, persist. Unknown ids are skipped with a log.
     */
    public boolean switchTo(String profileId) {
        ConfigManager.LoadedDocument doc =
                config.load("profiles/" + sanitize(profileId) + ".json");
        if (doc.source() == ConfigManager.LoadSource.FRESH && !java.nio.file.Files.exists(
                profilesDir.resolve(sanitize(profileId) + ".json"))) {
            LOG.error("profile '{}' does not exist", profileId);
            return false;
        }
        List<String> wanted = new ArrayList<>();
        JsonArray arr = doc.json().getAsJsonArray("enabled_modules");
        if (arr != null) {
            arr.forEach(e -> wanted.add(e.getAsString()));
        }

        // Phase 1: disable what's on but not wanted (listeners torn down first).
        for (Module m : modules.all()) {
            if (m.isEnabled() && !wanted.contains(m.id())) {
                modules.setEnabled(m.id(), false);
            }
        }
        // Phase 2: enable what's wanted but off.
        int skipped = 0;
        for (String id : wanted) {
            if (modules.byId(id) == null || !modules.setEnabled(id, true)) {
                skipped++;
                LOG.warn("profile '{}': module '{}' unknown or unavailable — skipped",
                        profileId, id);
            }
        }
        LOG.info("switched to profile '{}' ({} skipped entries)", profileId, skipped);
        return true;
    }

    private static String sanitize(String id) {
        String clean = id.replaceAll("[^a-zA-Z0-9_-]", "_");
        return clean.isEmpty() ? "default" : clean;
    }
}
