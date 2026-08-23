package net.isekaiyo.client.core.modules;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.events.Events;
import net.isekaiyo.client.core.settings.Setting;

/**
 * Base class for every Isekaiyo module (spec §7/§8/§34/§35).
 *
 * <p>Contributor contract:</p>
 * <ol>
 *   <li>extend {@link Module}, fill the constructor metadata</li>
 *   <li>declare {@link Setting}s as final fields via {@code StandardSettings}</li>
 *   <li>declare required capabilities / module dependencies in the ctor</li>
 *   <li>subscribe to {@link Events} buses in {@link #onEnable}, keep the
 *       returned subscriptions (they are torn down automatically on
 *       disable)</li>
 *   <li>register explicitly in {@code BuiltInModules} — no scanning magic</li>
 * </ol>
 *
 * <p>Performance rule (spec §44): a disabled module does NO work. The base
 * class enforces this by only dispatching lifecycle hooks when enabled.</p>
 */
public abstract class Module {

    private final String id;
    private final String name;
    private final String description;
    private final ModuleCategory category;
    private final Capability[] requiredCapabilities;
    private final String[] requiredModules;

    private boolean enabled;
    /** Set by the manager when capabilities/deps are unmet; never toggled by users. */
    private boolean unavailable;
    /** Lazily built once — settings discovery must not reflect every call (spec §44). */
    private List<Setting<?>> settingsCache;

    protected Module(String id, String name, String description, ModuleCategory category) {
        this(id, name, description, category, new Capability[0], new String[0]);
    }

    protected Module(
            String id,
            String name,
            String description,
            ModuleCategory category,
            Capability[] requiredCapabilities,
            String[] requiredModules) {
        this.id = id;
        this.name = name;
        this.description = description;
        this.category = category;
        this.requiredCapabilities = requiredCapabilities;
        this.requiredModules = requiredModules;
    }

    // ---- metadata ----------------------------------------------------------

    public final String id() {
        return id;
    }

    public final String name() {
        return name;
    }

    public final String description() {
        return description;
    }

    public final ModuleCategory category() {
        return category;
    }

    public final boolean isEnabled() {
        return enabled && !unavailable;
    }

    public final boolean isUnavailable() {
        return unavailable;
    }

    final void setUnavailable(boolean unavailable) {
        this.unavailable = unavailable;
        if (unavailable) {
            this.enabled = false;
        }
    }

    public final List<Setting<?>> settings() {
        if (settingsCache != null) {
            return settingsCache;
        }
        List<Setting<?>> found = new ArrayList<>();
        for (java.lang.reflect.Field field : getClass().getDeclaredFields()) {
            if (java.lang.reflect.Modifier.isStatic(field.getModifiers())) {
                continue;
            }
            if (!Setting.class.isAssignableFrom(field.getType())) {
                continue;
            }
            field.setAccessible(true);
            try {
                found.add((Setting<?>) field.get(this));
            } catch (IllegalAccessException e) {
                throw new IllegalStateException(
                        "module " + id + " has an inaccessible setting field", e);
            }
        }
        settingsCache = Collections.unmodifiableList(found);
        return settingsCache;
    }

    public final Setting<?> setting(String settingId) {
        for (Setting<?> s : settings()) {
            if (s.id().equals(settingId)) {
                return s;
            }
        }
        return null;
    }

    public final Capability[] requiredCapabilities() {
        return requiredCapabilities;
    }

    public final String[] requiredModules() {
        return requiredModules;
    }

    // ---- state transitions (called by ModuleManager only) ------------------

    final void enable() {
        if (enabled || unavailable) {
            return;
        }
        enabled = true;
        try {
            onEnable();
        } catch (Throwable t) {
            enabled = false;
            throw t;
        }
    }

    final void disable() {
        if (!enabled) {
            return;
        }
        enabled = false;
        // Subscriptions are removed by the manager BEFORE this call; onDisable
        // is for releasing other resources.
        try {
            onDisable();
        } catch (Throwable t) {
            // Never let a broken onDisable block shutdown or profile switches.
            // The manager logs it.
        }
    }

    // ---- lifecycle hooks (spec §8 — few, event-driven) ----------------------

    /** Acquire resources + subscribe to events. Throwing aborts the enable. */
    protected void onEnable() {}

    /** Release resources. Subscriptions are already gone. */
    protected void onDisable() {}

    /** One-time registration-time hook (e.g. building static state). */
    protected void onLoad() {}

    /** Symmetric to {@link #onLoad()}; called at client shutdown. */
    protected void onUnload() {}
}
