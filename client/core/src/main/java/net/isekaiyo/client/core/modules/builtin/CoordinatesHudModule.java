package net.isekaiyo.client.core.modules.builtin;

import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.IsekaiyoClient;
import net.isekaiyo.client.core.VersionAdapter;
import net.isekaiyo.client.core.hud.HudElement;
import net.isekaiyo.client.core.hud.HudPosition;
import net.isekaiyo.client.core.hud.HudRenderer;
import net.isekaiyo.client.core.modules.Module;
import net.isekaiyo.client.core.modules.ModuleCategory;
import net.isekaiyo.client.core.settings.Setting;
import net.isekaiyo.client.core.settings.StandardSettings;

/**
 * Player coordinates overlay (spec §22). Gracefully renders nothing without
 * a player/world; shows the dimension and nether-coordinate context.
 */
public final class CoordinatesHudModule extends Module implements HudElement {

    private final Setting<Boolean> showDimension =
            StandardSettings.bool("show_dimension", true);
    private final Setting<Boolean> showNetherEquivalent =
            StandardSettings.bool("nether_equivalent", false);

    private final VersionAdapter adapter;

    public CoordinatesHudModule(VersionAdapter adapter) {
        super(
                "coordinates",
                "Coordinates",
                "Shows your position and current dimension.",
                ModuleCategory.HUD,
                Capability.of(Capability.HUD, Capability.PLAYER_STATE),
                new String[0]);
        this.adapter = adapter;
    }

    @Override
    public String id() {
        return "hud.coordinates";
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
        return 24;
    }

    @Override
    public float scale() {
        return 1.0f;
    }

    @Override
    public int layer() {
        return 10;
    }

    @Override
    public int estimatedWidth(HudRenderer renderer) {
        return renderer.textWidth("X: -12345  Y: -64  Z: -12345  (overworld)") + 8;
    }

    @Override
    public int estimatedHeight() {
        return 14;
    }

    @Override
    public void render(HudRenderer renderer) {
        // Absent state → draw nothing, never crash (spec §22).
        double[] pos = adapter.playerPosition();
        if (pos == null || pos.length != 3) {
            return;
        }
        String dimension = adapter.dimensionId();
        StringBuilder text = new java.lang.StringBuilder(64);
        text.append("X: ").append((int) Math.floor(pos[0]))
                .append("  Y: ").append((int) Math.floor(pos[1]))
                .append("  Z: ").append((int) Math.floor(pos[2]));
        if (showDimension.get() && dimension != null) {
            text.append("  (").append(dimension).append(')');
        }
        if (showNetherEquivalent.get()) {
            double factor = "the_nether".equals(dimension) ? 8.0 : ("overworld".equals(dimension) ? 1.0 / 8.0 : 0.0);
            if (factor > 0) {
                text.append(String.format("  [%d, %d]",
                        (int) Math.floor(pos[0] * factor),
                        (int) Math.floor(pos[2] * factor)));
            }
        }
        int[] p = topLeft(renderer);
        var theme = IsekaiyoClient.get().context().theme();
        renderer.panel(theme, p[0], p[1], estimatedWidth(renderer), estimatedHeight());
        renderer.drawTextWithShadow(text.toString(), p[0] + 4, p[1] + 3, theme.textArgb());
    }
}
