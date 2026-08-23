package net.isekaiyo.client.core.config;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.LinkedHashMap;
import java.util.Map;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * The client configuration system (spec §14/§15/§57).
 *
 * <p>Files live under {@code <gameDir>/config/isekaiyo/}:</p>
 * <pre>
 *   client.json      — global client options
 *   modules.json     — enabled state + settings per module
 *   hud.json         — element layout
 *   keybinds.json    — keybind assignments
 *   profiles/*.json  — named full configurations
 * </pre>
 *
 * <p>Every document carries {@code config_version}. Reading is defensive:
 * missing fields keep defaults, unknown fields are ignored, invalid values
 * are rejected per-setting, and a completely unparseable file is backed up
 * (never deleted) before defaults load (spec §57).</p>
 */
public final class ConfigManager {

    public static final int CURRENT_CONFIG_VERSION = 1;

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo/Config");
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create();

    private final Path root;
    private final Migrations migrations;

    public ConfigManager(Path configRoot) {
        this.root = configRoot;
        this.migrations = new Migrations();
    }

    public Path root() {
        return root;
    }

    // ---- reading ------------------------------------------------------------

    /**
     * Load a JSON document, migrating and recovering as needed. Returns the
     * (possibly migrated) object, or a fresh empty object when the file was
     * absent or unrecoverable. The caller never sees a null.
     */
    public LoadedDocument load(String fileName) {
        Path file = root.resolve(fileName);
        if (!Files.exists(file)) {
            return new LoadedDocument(new JsonObject(), LoadSource.FRESH, null);
        }
        String raw;
        try {
            raw = Files.readString(file, StandardCharsets.UTF_8);
        } catch (IOException e) {
            LOG.error("cannot read config file {} — defaults in effect", file, e);
            return new LoadedDocument(new JsonObject(), LoadSource.ERROR, null);
        }

        JsonObject json;
        try {
            json = JsonParser.parseString(raw).getAsJsonObject();
        } catch (Exception e) {
            Path backup = backupCorrupt(file);
            LOG.error(
                    "config file {} is not valid JSON — preserved at {}; defaults in effect",
                    file,
                    backup,
                    e
            );
            return new LoadedDocument(new JsonObject(), LoadSource.CORRUPT, backup);
        }

        // Version migration chain (spec §15).
        int version = json.has("config_version")
                ? json.get("config_version").getAsInt()
                : 1;
        if (version < CURRENT_CONFIG_VERSION) {
            json = migrations.migrate(json, version);
            LOG.info("migrated {} from config_version {} to {}", fileName, version,
                    CURRENT_CONFIG_VERSION);
        } else if (version > CURRENT_CONFIG_VERSION) {
            // Future config read by older client: keep going with what we
            // understand rather than refusing (forward-compatible fields are
            // ignored by design).
            LOG.warn("{} was written by a newer Isekaiyo (config_version {}); loading anyway",
                    fileName, version);
        }
        return new LoadedDocument(json, LoadSource.OK, null);
    }

    private Path backupCorrupt(Path file) {
        Path backup = file.resolveSibling(file.getFileName() + ".corrupt");
        try {
            Files.move(file, backup, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException e) {
            LOG.error("could not preserve corrupt config {}", file, e);
            return file; // report honestly even when preservation failed
        }
        return backup;
    }

    // ---- writing ---------------------------------------------------------------

    /** Atomic write: temp file + move, so a crash never truncates config. */
    public void save(String fileName, JsonObject document) {
        try {
            Files.createDirectories(root);
            document.addProperty("config_version", CURRENT_CONFIG_VERSION);
            Path file = root.resolve(fileName);
            Path tmp = file.resolveSibling(file.getFileName() + ".tmp");
            Files.writeString(tmp, GSON.toJson(document), StandardCharsets.UTF_8);
            Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException e) {
            // Never fatal: log loudly; the next save will retry.
            LOG.error("failed to save config {}", fileName, e);
        }
    }

    /** Convenience for building section documents. */
    public static JsonObject object() {
        return new JsonObject();
    }

    /** Ordered map helper (keeps config diffs stable). */
    public static Map<String, Object> orderedMap() {
        return new LinkedHashMap<>();
    }

    public enum LoadSource {
        FRESH,
        OK,
        CORRUPT,
        ERROR
    }

    /** Result of {@link #load(String)}. */
    public record LoadedDocument(JsonObject json, LoadSource source, Path corruptBackupPath) {}
}
