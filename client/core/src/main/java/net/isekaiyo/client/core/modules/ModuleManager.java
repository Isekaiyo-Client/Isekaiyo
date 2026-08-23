package net.isekaiyo.client.core.modules;

import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.isekaiyo.client.core.VersionAdapter;
import net.isekaiyo.client.core.events.Events;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Owns module registration and state (spec §9). The ONLY way modules change
 * enabled state; UI and other subsystems go through here.
 *
 * <p>Unavailable handling (spec §35): at registration each module is checked
 * against the adapter's capabilities and against registered dependencies.
 * Unmet ⇒ the module is marked unavailable and can never be enabled — it will
 * still appear in listings so users understand why it's absent.</p>
 */
public final class ModuleManager {

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo/Module");

    private final Map<String, Module> modules = new LinkedHashMap<>();
    private final VersionAdapter adapter;
    private final ConfigSink configSink;

    /** Where the manager persists module enabled-state + settings. */
    public interface ConfigSink {
        void saveModules(Collection<Module> modules);

        void loadModules(Collection<Module> modules);
    }

    public ModuleManager(VersionAdapter adapter, ConfigSink configSink) {
        this.adapter = adapter;
        this.configSink = configSink;
    }

    // ---- registration -------------------------------------------------------

    /**
     * Explicit registration (spec §48). Duplicate ids are a programming error.
     * Returns this for fluent registration blocks.
     */
    public ModuleManager register(Module module) {
        if (modules.containsKey(module.id())) {
            throw new IllegalArgumentException("duplicate module id: " + module.id());
        }
        evaluateAvailability(module);
        modules.put(module.id(), module);
        try {
            module.onLoad();
        } catch (Throwable t) {
            LOG.error("module {} threw during onLoad", module.id(), t);
        }
        return this;
    }

    private void evaluateAvailability(Module module) {
        for (Capability cap : module.requiredCapabilities()) {
            if (!adapter.hasCapability(cap)) {
                module.setUnavailable(true);
                LOG.info(
                        "module {} unavailable: adapter lacks capability {}",
                        module.id(),
                        cap
                );
                return;
            }
        }
        // Dependencies must already be registered AND available.
        for (String depId : module.requiredModules()) {
            Module dep = modules.get(depId);
            boolean ok = dep != null && !dep.isUnavailable();
            if (!ok) {
                module.setUnavailable(true);
                LOG.info(
                        "module {} unavailable: dependency '{}' missing or unavailable",
                        module.id(),
                        depId
                );
                return;
            }
        }
    }

    // ---- queries -------------------------------------------------------------

    public Module byId(String id) {
        return modules.get(id);
    }

    /** All modules, stable order (registration then name). */
    public List<Module> all() {
        List<Module> list = new ArrayList<>(modules.values());
        list.sort(Comparator.comparing(Module::name));
        return list;
    }

    /** Case-insensitive metadata search across id/name/description (spec §28). */
    public List<Module> search(String query) {
        String q = query == null ? "" : query.trim().toLowerCase();
        if (q.isEmpty()) {
            return all();
        }
        List<Module> hits = new ArrayList<>();
        for (Module m : all()) {
            if (m.name().toLowerCase().contains(q)
                    || m.id().toLowerCase().contains(q)
                    || m.description().toLowerCase().contains(q)) {
                hits.add(m);
            }
        }
        return hits;
    }

    /** Categories that actually contain modules (spec §29). */
    public List<ModuleCategory> presentCategories() {
        Map<ModuleCategory, Boolean> seen = new LinkedHashMap<>();
        for (Module m : modules.values()) {
            seen.putIfAbsent(m.category(), true);
        }
        List<ModuleCategory> cats = new ArrayList<>(seen.keySet());
        cats.sort(Comparator.comparing(Enum::name));
        return Collections.unmodifiableList(cats);
    }

    public int enabledCount() {
        return (int) modules.values().stream().filter(Module::isEnabled).count();
    }

    // ---- state changes ---------------------------------------------------------

    public boolean setEnabled(String moduleId, boolean enable) {
        Module m = modules.get(moduleId);
        if (m == null || m.isUnavailable()) {
            return false;
        }
        if (enable && !m.isEnabled()) {
            m.enable();
            LOG.info("module {} enabled", moduleId);
            persist();
            return true;
        }
        if (!enable && m.isEnabled()) {
            Events.unsubscribeOwner(moduleId); // listeners die first (spec §10)
            m.disable();
            LOG.info("module {} disabled", moduleId);
            persist();
            return true;
        }
        return false;
    }

    public boolean toggle(String moduleId) {
        Module m = modules.get(moduleId);
        return m != null && setEnabled(moduleId, !m.isEnabled());
    }

    private void persist() {
        try {
            configSink.saveModules(modules.values());
        } catch (Exception e) {
            LOG.error("failed to persist module state", e);
        }
    }

    // ---- bulk operations --------------------------------------------------------

    /** Disable everything safely: listeners removed, onDisable isolated. */
    public void disableAll() {
        for (Module m : modules.values()) {
            if (m.isEnabled()) {
                Events.unsubscribeOwner(m.id());
                m.disable();
            }
        }
    }

    public void loadPersistedState() {
        try {
            configSink.loadModules(modules.values());
        } catch (Exception e) {
            // Corrupt module config must never brick the client (spec §33/§57).
            LOG.error("module config unreadable — defaults in effect", e);
        }
    }

    /** Called once during client shutdown. */
    public void unloadAll() {
        disableAll();
        for (Module m : modules.values()) {
            try {
                m.onUnload();
            } catch (Throwable t) {
                LOG.error("module {} threw during onUnload", m.id(), t);
            }
        }
    }
}
