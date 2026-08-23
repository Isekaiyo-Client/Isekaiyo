package net.isekaiyo.client.core.modules.builtin;

import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.IsekaiyoClient;
import net.isekaiyo.client.core.events.Events;
import net.isekaiyo.client.core.hud.HudElement;
import net.isekaiyo.client.core.hud.HudPosition;
import net.isekaiyo.client.core.hud.HudRenderer;
import net.isekaiyo.client.core.modules.Module;
import net.isekaiyo.client.core.modules.ModuleCategory;
import net.isekaiyo.client.core.settings.Setting;
import net.isekaiyo.client.core.settings.StandardSettings;

/**
 * WASD + mouse keystrokes overlay (spec §23). Reacts to REAL input through
 * the adapter-forwarded {@code KeyInputEvent}/{@code MouseInputEvent}; state
 * resets on world leave so stale presses never linger.
 */
public final class KeystrokesModule extends Module implements HudElement {

    // GLFW codes for W/A/S/D (see KeybindManager table).
    private static final int KEY_W = 'W';
    private static final int KEY_A = 'A';
    private static final int KEY_S = 'S';
    private static final int KEY_D = 'D';

    private static final int CELL = 22;
    private static final int GAP = 2;

    private final Setting<Boolean> showMouse =
            StandardSettings.bool("show_mouse", true);
    private final Setting<Integer> opacityPercent =
            StandardSettings.integer("opacity_percent", 80, 20, 100);

    private final VersionAdapter adapter;

    private volatile boolean w;
    private volatile boolean a;
    private volatile boolean s;
    private volatile boolean d;
    private volatile boolean lmb;
    private volatile boolean rmb;

    public KeystrokesModule(VersionAdapter adapter) {
        super(
                "keystrokes",
                "Keystrokes",
                "Shows live WASD and mouse button input.",
                ModuleCategory.HUD,
                Capability.of(Capability.HUD, Capability.INPUT),
                new String[0]);
        this.adapter = adapter;
    }

    @Override
    protected void onEnable() {
        Events.KEY_INPUT.subscribe(id(), this::onKey);
        Events.MOUSE_INPUT.subscribe(id(), this::onMouse);
        Events.WORLD_LEAVE.subscribe(id(), e -> clear());
    }

    @Override
    protected void onDisable() {
        w = a = s = d = lmb = rmb = false;
    }

    private void onKey(Events.KeyInputEvent e) {
        switch (e.keyCode()) {
            case KEY_W -> w = e.isPress();
            case KEY_A -> a = e.isPress();
            case KEY_S -> s = e.isPress();
            case KEY_D -> d = e.isPress();
            default -> { /* not ours */ }
        }
    }

    private void onMouse(Events.MouseInputEvent e) {
        if (e.button() == 0) {
            lmb = e.isPress();
        } else if (e.button() == 1) {
            rmb = e.isPress();
        }
    }

    private void clear() {
        w = a = s = d = lmb = rmb = false;
    }

    @Override
    public String id() {
        return "hud.keystrokes";
    }

    @Override
    public Module module() {
        return this;
    }

    @Override
    public HudPosition position() {
        return HudPosition.BOTTOM_LEFT;
    }

    @Override
    public int offsetX() {
        return 8;
    }

    @Override
    public int offsetY() {
        return 40;
    }

    @Override
    public float scale() {
        return 1.0f;
    }

    @Override
    public int layer() {
        return 11;
    }

    @Override
    public int estimatedWidth(HudRenderer renderer) {
        return showMouse.get() ? CELL * 3 + GAP * 2 : CELL + GAP * 2;
    }

    @Override
    public int estimatedHeight() {
        int rows = showMouse.get() ? 4 : 3;
        return rows * CELL + (rows - 1) * GAP;
    }

    @Override
    public void render(HudRenderer renderer) {
        var theme = IsekaiyoClient.get().context().theme();
        int alphaBase = (opacityPercent.get() << 24); // scale accent alpha
        int idleBg = (alphaBase & 0xFF000000) | 0x101014;
        int pressedBg = theme.accentArgb();
        int textColor = theme.textArgb();

        int[] origin = topLeft(renderer);
        int x = origin[0];
        int y = origin[1];

        // Row 1: W
        cell(renderer, x + CELL + GAP, y, CELL, CELL, w, "W", pressedBg, idleBg, textColor);
        // Row 2: A S D
        int row2 = y + CELL + GAP;
        cell(renderer, x, row2, CELL, CELL, a, "A", pressedBg, idleBg, textColor);
        cell(renderer, x + CELL + GAP, row2, CELL, CELL, s, "S", pressedBg, idleBg, textColor);
        cell(renderer, x + (CELL + GAP) * 2, row2, CELL, CELL, d, "D", pressedBg, idleBg, textColor);

        if (showMouse.get()) {
            int row3 = row2 + CELL + GAP;
            cell(renderer, x, row3, CELL, CELL, lmb, "LMB", pressedBg, idleBg, textColor);
            cell(renderer, x + CELL + GAP, row3, CELL, CELL, rmb, "RMB", pressedBg, idleBg, textColor);
        }
    }

    private static void cell(
            HudRenderer r,
            int x,
            int y,
            int w,
            int h,
            boolean pressed,
            String label,
            int pressedColor,
            int idleColor,
            int textColor) {
        r.fill(x, y, x + w, y + h, pressed ? pressedColor : idleColor);
        int textWidth = r.textWidth(label);
        r.drawText(label, x + ((w - textWidth) / 2), y + ((h - 8) / 2), pressed ? 0xFFFFFFFF : textColor);
    }
}
