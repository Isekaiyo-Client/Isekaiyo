package net.isekaiyo.client.core.hud;

import net.isekaiyo.client.core.Capability;
import net.isekaiyo.client.core.modules.Module;

/**
 * A movable HUD widget backed by a module (spec §18/§19). The module owns
 * enable state + settings; the element owns placement and drawing.
 *
 * <p>Performance contract (spec §21/§44): {@link #render} runs every frame —
 * no allocations, no filesystem, no string building unless the value actually
 * changed (see FpsHudModule's cached text).</p>
 */
public interface HudElement {

    String id();

    /** Owning module; provides enabled state + settings. */
    Module module();

    /** Screen anchor. */
    HudPosition position();

    /** Offset in GUI pixels from the anchor. */
    int offsetX();

    int offsetY();

    /** Render scale multiplier (0.5–3). */
    float scale();

    /** Draw order within overlapping elements; lower draws first. */
    int layer();

    /**
     * Compute the top-left pixel for this element given its anchor and
     * offsets. Elements that need centering use the screen size.
     */
    default int[] topLeft(HudRenderer renderer) {
        int x = switch (position()) {
            case TOP_LEFT, BOTTOM_LEFT -> offsetX();
            case TOP_CENTER, BOTTOM_CENTER ->
                    (renderer.screenWidth() / 2) - (estimatedWidth(renderer) / 2) + offsetX();
            case TOP_RIGHT, BOTTOM_RIGHT ->
                    renderer.screenWidth() - estimatedWidth(renderer) - offsetX();
        };
        int y = position().isBottom()
                ? renderer.screenHeight() - estimatedHeight() - offsetY()
                : offsetY();
        return new int[] {x, y};
    }

    /** Cheap width estimate for anchoring; elements may override precisely. */
    default int estimatedWidth(HudRenderer renderer) {
        return 60;
    }

    default int estimatedHeight() {
        return 12;
    }

    /**
     * Draw this frame. Called only when the owning module is enabled and the
     * HUD capability exists.
     */
    void render(HudRenderer renderer);

    /** Capability needed beyond HUD itself; usually empty. */
    default Capability[] extraRequirements() {
        return new Capability[0];
    }
}
