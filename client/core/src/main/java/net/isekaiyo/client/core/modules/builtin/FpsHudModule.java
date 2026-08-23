package net.isekaiyo.client.core.modules.builtin;

import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.VersionAdapter;
import net.isekaiyo.client.core.hud.HudElement;
import net.isekaiyo.client.core.hud.HudPosition;
import net.isekaiyo.client.core.hud.HudRenderer;
import net.isekaiyo.client.core.modules.Module;
import net.isekaiyo.client.core.modules.ModuleCategory;
import net.isekaiyo.client.core.settings.Setting;
import net.isekaiyo.client.core.settings.StandardSettings;

/**
 * FPS overlay (spec §21). Demonstrates the full chain: module → event
 * subscription → typed settings → HUD element → theme tokens.
 *
 * <p>Allocation discipline: the label string is rebuilt at most once per
 * second (value-change check), never per frame.</p>
 */
public final class FpsHudModule extends Module implements HudElement {

    private final Setting<Boolean> showLabel =
            StandardSettings.bool("show_label", true);
    private final Setting<Integer> scalePercent =
            StandardSettings.integer("scale_percent", 100, 50, 300);

    private final VersionAdapter adapter;

    private int lastFps = -1;
    private String cachedText = "";

    public FpsHudModule(VersionAdapter adapter) {
        super(
                "fps",
                "FPS",
                "Shows the current frames-per-second.",
                ModuleCategory.HUD,
                Capability.of(Capability.HUD),
                new String[0]);
        this.adapter = adapter;
    }

    // Rendering happens via HudElement through HudManager — no event
    // subscription needed for this module.
    // ---- HudElement ---------------------------------------------------------

    @Override
    public String id() {
        return "hud.fps";
    }

    @Override
    public Module module() {
        return this;
    }

    @Override
    public HudPosition position() {
        return HudPosition.TOP_LEFT;
    }

    @Override
    public int offsetX() {
        return 6;
    }

    @Override
    public int offsetY() {
        return 6;
    }

    @Override
    public float scale() {
        return scalePercent.get() / 100.0f;
    }

    @Override
    public int layer() {
        return 10;
    }

    @Override
    public int estimatedWidth(HudRenderer renderer) {
        return renderer.textWidth(cachedText.isEmpty() ? "00 FPS" : cachedText) + 8;
    }

    @Override
    public int estimatedHeight() {
        return 14;
    }

    @Override
    public void render(HudRenderer renderer) {
        int fps = adapter.currentFps();
        if (fps != lastFps) {
            lastFps = fps;
            cachedText = showLabel.get() ? fps + " FPS" : String.valueOf(fps);
        }
        if (cachedText.isEmpty()) {
            cachedText = showLabel.get() ? fps + " FPS" : String.valueOf(fps);
        }
        int[] pos = topLeft(renderer);
        renderer.panel(
                net.isekaiyo.client.core.IsekaiyoClient.get().context().theme(),
                pos[0], pos[1],
                estimatedWidth(renderer),
                estimatedHeight());
        renderer.drawTextWithShadow(
                cachedText,
                pos[0] + 4,
                pos[1] + 3,
                net.isekaiyo.client.core.IsekaiyoClient.get().context().theme().textArgb());
    }
}
