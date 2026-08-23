package net.isekaiyo.client.fabric;

import net.isekaiyo.client.core.hud.HudRenderer;
import net.minecraft.client.gui.DrawContext;

/**
 * {@link HudRenderer} over the modern (1.20+) {@link DrawContext} API.
 * Rebound to each frame's context by the adapter; drawing outside a bound
 * frame is refused rather than crashing on stale state.
 */
public final class FabricHudRenderer implements HudRenderer {

    private DrawContext context;
    private int screenWidth;
    private int screenHeight;

    /** Bind to this frame's context (called once per frame by the adapter). */
    public void bind(Object drawContext) {
        this.context = (DrawContext) drawContext;
        var mc = net.minecraft.client.MinecraftClient.getInstance();
        var window = mc.getWindow();
        // GUI-scaled dimensions match what DrawContext coordinates expect.
        this.screenWidth = window.getScaledWidth();
        this.screenHeight = window.getScaledHeight();
    }

    public boolean isBound() {
        return context != null;
    }

    @Override
    public int screenWidth() {
        return screenWidth;
    }

    @Override
    public int screenHeight() {
        return screenHeight;
    }

    @Override
    public int drawText(String text, int x, int y, int argb) {
        if (context == null) {
            return 0;
        }
        var matrices = context.getMatrices();
        context.drawText(
                net.minecraft.client.MinecraftClient.getInstance().textRenderer,
                text,
                x,
                y,
                argb,
                false);
        return textWidth(text);
    }

    @Override
    public int drawTextWithShadow(String text, int x, int y, int argb) {
        if (context == null) {
            return 0;
        }
        context.drawText(
                net.minecraft.client.MinecraftClient.getInstance().textRenderer,
                text,
                x,
                y,
                argb,
                true);
        return textWidth(text);
    }

    @Override
    public int textWidth(String text) {
        if (context == null || text == null || text.isEmpty()) {
            return 0;
        }
        return net.minecraft.client.MinecraftClient.getInstance()
                .textRenderer.getWidth(text);
    }

    @Override
    public void fill(int x1, int y1, int x2, int y2, int argb) {
        if (context != null) {
            context.fill(x1, y1, x2, y2, argb);
        }
    }

    /** Release the frame reference so nothing draws outside its frame. */
    public void unbind() {
        this.context = null;
    }
}
