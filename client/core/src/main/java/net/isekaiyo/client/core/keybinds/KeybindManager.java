package net.isekaiyo.client.core.keybinds;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Keybind model + manager (spec §16/§17). Keybinds bind ACTIONS (usually
 * module toggles) to keys — modules never hard-code keys.
 *
 * <p>Key encoding: GLFW key code, or negative values for mouse buttons
 * ({@code MOUSE_BUTTON_1 = -1}, …), and {@code UNBOUND = -100}.</p>
 */
public final class KeybindManager {

    public static final int UNBOUND = -100;
    public static final int MOUSE_BUTTON_1 = -1;

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo/Input");

    /** One binding: an action id (e.g. {@code "module.toggleSprint"}) → key. */
    public record Binding(String actionId, String displayName, int keyCode) {
        public Binding {
            Objects.requireNonNull(actionId);
            Objects.requireNonNull(displayName);
        }

        public boolean isUnbound() {
            return keyCode == UNBOUND;
        }

        public String describeKey() {
            if (isUnbound()) {
                return "unbound";
            }
            if (keyCode < 0) {
                return "Mouse " + (-keyCode);
            }
            String name = GLFW_NAMES.get(keyCode);
            return name != null ? name : ("Key " + keyCode);
        }
    }

    private final Map<String, Binding> byAction = new HashMap<>();

    /** Register (or re-register) an action's binding. */
    public void bind(String actionId, String displayName, int keyCode) {
        byAction.put(actionId, new Binding(actionId, displayName, keyCode));
    }

    public Binding bindingFor(String actionId) {
        return byAction.get(actionId);
    }

    public List<Binding> all() {
        return new ArrayList<>(byAction.values());
    }

    public void unbind(String actionId) {
        Binding current = byAction.get(actionId);
        if (current != null) {
            byAction.put(actionId,
                    new Binding(current.actionId(), current.displayName(), UNBOUND));
        }
    }

    /**
     * Conflicts (spec §17): actions sharing a bound key. Reported, not
     * forbidden — intentional sharing is legitimate (e.g. hold-vs-toggle on
     * the same key).
     */
    public List<Conflict> conflicts() {
        Map<Integer, List<Binding>> byKey = new HashMap<>();
        for (Binding b : byAction.values()) {
            if (!b.isUnbound()) {
                byKey.computeIfAbsent(b.keyCode(), k -> new ArrayList<>()).add(b);
            }
        }
        List<Conflict> conflicts = new ArrayList<>();
        for (List<Binding> group : byKey.values()) {
            if (group.size() > 1) {
                conflicts.add(new Conflict(List.copyOf(group)));
            }
        }
        return conflicts;
    }

    public record Conflict(List<Binding> bindings) {
        public String describe() {
            List<String> names = bindings.stream().map(b -> b.actionId()).toList();
            return bindings.get(0).describeKey() + " is shared by: " + String.join(", ", names);
        }
    }

    /** Resolve which action a raw key event targets, if any. */
    public Binding actionForKey(int keyCode, boolean isPress) {
        if (!isPress) {
            return null;
        }
        for (Binding b : byAction.values()) {
            if (b.keyCode() == keyCode) {
                return b;
            }
        }
        return null;
    }

    /** Minimal GLFW-name table for common keys; unknown codes render numerically. */
    private static final Map<Integer, String> GLFW_NAMES = Map.ofEntries(
            Map.entry(32, "SPACE"),
            Map.entry(39, "APOSTROPHE"),
            Map.entry(44, "COMMA"),
            Map.entry(45, "MINUS"),
            Map.entry(46, "PERIOD"),
            Map.entry(59, "F1"),
            Map.entry(60, "F2"),
            Map.entry(61, "F3"),
            Map.entry(62, "F4"),
            Map.entry(63, "F5"),
            Map.entry(64, "F6"),
            Map.entry(65, "F7"),
            Map.entry(66, "F8"),
            Map.entry(67, "F9"),
            Map.entry(68, "F10"),
            Map.entry(87, "F11"),
            Map.entry(88, "F12"),
            Map.entry(256, "ESC"),
            Map.entry(257, "ENTER"),
            Map.entry(258, "TAB"),
            Map.entry(259, "BACKSPACE"),
            Map.entry(260, "INSERT"),
            Map.entry(261, "DELETE"),
            Map.entry(262, "RIGHT"),
            Map.entry(263, "LEFT"),
            Map.entry(264, "DOWN"),
            Map.entry(265, "UP"),
            Map.entry(280, "CAPS_LOCK"),
            Map.entry(340, "LEFT_SHIFT"),
            Map.entry(341, "LEFT_CTRL"),
            Map.entry(342, "LEFT_ALT"),
            Map.entry(344, "RIGHT_SHIFT"),
            Map.entry(65 + 0, "A"),
            Map.entry(65 + 1, "B"),
            Map.entry(65 + 2, "C"),
            Map.entry(65 + 3, "D"),
            Map.entry(65 + 4, "E"),
            Map.entry(65 + 5, "F"),
            Map.entry(65 + 6, "G"),
            Map.entry(65 + 7, "H"),
            Map.entry(65 + 8, "I"),
            Map.entry(65 + 9, "J"),
            Map.entry(65 + 10, "K"),
            Map.entry(65 + 11, "L"),
            Map.entry(65 + 12, "M"),
            Map.entry(65 + 13, "N"),
            Map.entry(65 + 14, "O"),
            Map.entry(65 + 15, "P"),
            Map.entry(65 + 16, "Q"),
            Map.entry(65 + 17, "R"),
            Map.entry(65 + 18, "S"),
            Map.entry(65 + 19, "T"),
            Map.entry(65 + 20, "U"),
            Map.entry(65 + 21, "V"),
            Map.entry(65 + 22, "W"),
            Map.entry(65 + 23, "X"),
            Map.entry(65 + 24, "Y"),
            Map.entry(65 + 25, "Z"),
            Map.entry(48 + 0, "0"),
            Map.entry(48 + 1, "1"),
            Map.entry(48 + 2, "2"),
            Map.entry(48 + 3, "3"),
            Map.entry(48 + 4, "4"),
            Map.entry(48 + 5, "5"),
            Map.entry(48 + 6, "6"),
            Map.entry(48 + 7, "7"),
            Map.entry(48 + 8, "8"),
            Map.entry(48 + 9, "9")
    );
}
