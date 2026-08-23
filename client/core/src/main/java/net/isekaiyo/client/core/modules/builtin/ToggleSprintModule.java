package net.isekaiyo.client.core.modules.builtin;

import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.IsekaiyoClient;
import net.isekaiyo.client.core.VersionAdapter;
import net.isekaiyo.client.core.events.Events;
import net.isekaiyo.client.core.modules.Module;
import net.isekaiyo.client.core.modules.ModuleCategory;
import net.isekaiyo.client.core.notify.NotificationManager;
import net.isekaiyo.client.core.settings.Setting;
import net.isekaiyo.client.core.settings.StandardSettings;

/**
 * ToggleSprint (spec §50): a safe QOL movement preference. Sprint is a
 * client-side movement option Minecraft itself exposes — this changes
 * nothing the server does not already sanction. No reach, speed, or packet
 * behavior (spec §51).
 *
 * <p>Demonstrates keybind-driven toggling through the bus rather than a
 * per-module input hook.</p>
 */
public final class ToggleSprintModule extends Module {

    /** Default bind: G (GLFW 71). Rebindable via KeybindManager. */
    public static final String ACTION_ID = "module.toggle_sprint";

    private final Setting<Boolean> notifyOnChange =
            StandardSettings.bool("notify", true);
    private final Setting<Boolean> startEnabled =
            StandardSettings.bool("active_on_world_join", false);

    private final VersionAdapter adapter;
    private boolean sprintWanted;

    public ToggleSprintModule(VersionAdapter adapter) {
        super(
                "toggle_sprint",
                "Toggle Sprint",
                "Hold-free sprinting: press the bound key to keep sprinting.",
                ModuleCategory.MOVEMENT,
                Capability.of(Capability.PLAYER_STATE),
                new String[0]);
        this.adapter = adapter;
        this.sprintWanted = false;
    }

    @Override
    protected void onEnable() {
        sprintWanted = startEnabled.get();
        Events.KEY_INPUT.subscribe(id(), e -> {
            if (!e.isPress()) {
                return;
            }
            var binding = IsekaiyoClient.get().context().keybinds()
                    .bindingFor(ACTION_ID);
            if (binding != null && binding.keyCode() == e.keyCode()) {
                toggleSprint();
            }
        });
    }

    private void toggleSprint() {
        sprintWanted = !sprintWanted;
        if (notifyOnChange.get()) {
            IsekaiyoClient.get().context().notifications().push(
                    NotificationManager.Kind.INFO,
                    "Toggle Sprint",
                    sprintWanted ? "ON" : "OFF");
        }
        // The adapter applies the actual movement flag on the game thread.
        adapter.runOnGameThread(() -> { /* adapter-specific sprint flag */ });
    }

    /** Current preference, for adapter application each tick. */
    public boolean sprintWanted() {
        return isEnabled() && sprintWanted;
    }
}
