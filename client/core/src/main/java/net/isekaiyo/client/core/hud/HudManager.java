package net.isekaiyo.client.core.hud;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.isekaiyo.client.core.events.Events;

/**
 * Owns the HUD element set and performs the single per-frame render pass
 * (spec §18). Elements are registered by their modules at enable time.
 */
public final class HudManager {

    private final Map<String, HudElement> elements = new LinkedHashMap<>();

    public void register(HudElement element) {
        elements.put(element.id(), element);
    }

    public void unregister(String elementId) {
        elements.remove(elementId);
    }

    public HudElement byId(String id) {
        return elements.get(id);
    }

    public List<HudElement> all() {
        return new ArrayList<>(elements.values());
    }

    /**
     * One render pass: layer-ordered, only for ENABLED modules. Called from
     * the HUD_RENDER dispatch (adapter side) — game thread only.
     */
    public void render(Events.HudRenderEvent event) {
        List<HudElement> drawable = new ArrayList<>();
        for (HudElement e : elements.values()) {
            if (e.module().isEnabled()) {
                drawable.add(e);
            }
        }
        if (drawable.isEmpty()) {
            return; // zero work when nothing is on screen
        }
        drawable.sort(Comparator.comparingInt(HudElement::layer));
        for (HudElement e : drawable) {
            try {
                e.render(event.renderer());
            } catch (Throwable t) {
                // Crash isolation (spec §47): one broken element must not take
                // down the frame. Logged with the element id.
                org.slf4j.LoggerFactory.getLogger("Isekaiyo/Render")
                        .error("HUD element {} failed to render — hidden this frame", e.id(), t);
            }
        }
    }
}
