package net.isekaiyo.client.core.modules.builtin;

import net.isekaiyo.client.core.ClientContext;
import net.isekaiyo.client.core.IsekaiyoClient;

/**
 * THE registration point for built-in modules (spec §48/§49). A contributor
 * adds a module by writing its class and adding one line here — no scanning,
 * no config files, no magic.
 */
public final class BuiltInModules {

    private BuiltInModules() {}

    /** Register everything; called once between INITIALIZING and READY. */
    public static void registerAll(IsekaiyoClient client, ClientContext context) {
        var fps = new FpsHudModule(context.adapter());
        var coords = new CoordinatesHudModule(context.adapter());
        var keystrokes = new KeystrokesModule(context.adapter());
        var toggleSprint = new ToggleSprintModule(context.adapter());

        context.modules()
                .register(fps)
                .register(coords)
                .register(keystrokes)
                .register(toggleSprint);

        // HUD elements are registered alongside their modules.
        context.hud().register(fps);
        context.hud().register(coords);
        context.hud().register(keystrokes);

        // Default keybinds (rebindable; persisted by KeybindConfig).
        context.keybinds().bind(
                ToggleSprintModule.ACTION_ID,
                "Toggle Sprint",
                'G');
    }
}
