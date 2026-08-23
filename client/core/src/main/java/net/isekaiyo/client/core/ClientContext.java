package net.isekaiyo.client.core;

import net.isekaiyo.client.core.config.ConfigManager;
import net.isekaiyo.client.core.hud.HudManager;
import net.isekaiyo.client.core.keybinds.KeybindManager;
import net.isekaiyo.client.core.modules.ModuleManager;
import net.isekaiyo.client.core.notify.NotificationManager;
import net.isekaiyo.client.core.profiles.ProfileManager;
import net.isekaiyo.client.core.theme.ThemeTokens;

/**
 * The one object modules receive to reach subsystems (spec §4). It is a
 * narrow, explicit service locator — NOT a god singleton: it exposes the
 * seven managers plus the adapter and nothing else, and every field is
 * assigned exactly once during bootstrap.
 *
 * <p>Thread contract: safe to hold a reference from anywhere; individual
 * managers document their own threading rules.</p>
 */
public final class ClientContext {

    private final VersionAdapter adapter;
    private final ConfigManager config;
    private final ModuleManager modules;
    private final KeybindManager keybinds;
    private final HudManager hud;
    private final NotificationManager notifications;
    private final ProfileManager profiles;
    private volatile ThemeTokens theme = ThemeTokens.sakura();

    public ClientContext(
            VersionAdapter adapter,
            ConfigManager config,
            ModuleManager modules,
            KeybindManager keybinds,
            HudManager hud,
            NotificationManager notifications,
            ProfileManager profiles) {
        this.adapter = adapter;
        this.config = config;
        this.modules = modules;
        this.keybinds = keybinds;
        this.hud = hud;
        this.notifications = notifications;
        this.profiles = profiles;
    }

    public VersionAdapter adapter() {
        return adapter;
    }

    public ConfigManager config() {
        return config;
    }

    public ModuleManager modules() {
        return modules;
    }

    public KeybindManager keybinds() {
        return keybinds;
    }

    public HudManager hud() {
        return hud;
    }

    public NotificationManager notifications() {
        return notifications;
    }

    public ProfileManager profiles() {
        return profiles;
    }

    /** Current UI/HUD theme tokens; hot-swappable. */
    public ThemeTokens theme() {
        return theme;
    }

    public void setTheme(ThemeTokens tokens) {
        this.theme = tokens == null ? ThemeTokens.sakura() : tokens;
    }
}
