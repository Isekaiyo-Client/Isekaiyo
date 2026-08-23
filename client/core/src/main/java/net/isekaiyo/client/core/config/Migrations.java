package net.isekaiyo.client.core.config;

import com.google.gson.JsonObject;
import java.util.TreeMap;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Ordered migration chain (spec §15). Each entry upgrades a document from
 * key {@code from} to {@code from + 1}. Migrations are PURE transforms —
 * no I/O, fully unit-testable.
 *
 * <p>To change the config format next release: bump
 * {@link ConfigManager#CURRENT_CONFIG_VERSION}, add a step here, and write a
 * test feeding a v(n-1) document through.</p>
 */
final class Migrations {

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo/Config");

    /** from-version → transform. TreeMap so application order is deterministic. */
    private final TreeMap<Integer, Step> steps = new TreeMap<>();

    Migrations() {
        // v1 is the initial format; nothing to migrate yet. Example shape for
        // the first real migration:
        //
        // steps.put(1, doc -> {
        //     doc.remove("old_field");
        //     doc.addProperty("new_field", "default");
        //     return doc;
        // });
    }

    interface Step {
        JsonObject apply(JsonObject document);
    }

    JsonObject migrate(JsonObject document, int fromVersion) {
        JsonObject current = document;
        for (Map.Entry<Integer, Step> e : steps.tailMap(fromVersion).entrySet()) {
            try {
                current = e.getValue().apply(current);
            } catch (Exception ex) {
                // A failed migration keeps the document as-is rather than
                // destroying it; per-field defaults handle the rest.
                LOG.error("migration step {} failed — continuing with unmigrated data",
                        e.getKey(), ex);
                return current;
            }
        }
        current.addProperty("config_version", ConfigManager.CURRENT_CONFIG_VERSION);
        return current;
    }

    int stepCount() {
        return steps.size();
    }
}
