package net.isekaiyo.client.core.settings;

import java.util.Objects;

/**
 * One typed module setting with UI metadata (spec §13). Values are strongly
 * typed — never strings in a map — so modules read/write real values and the
 * future settings screen can render itself from this metadata alone.
 *
 * @param <T> the value type (Boolean, Integer, Float, String, enum, …)
 */
public abstract class Setting<T> {

    private final String id;
    private final String displayName;
    private final String description;
    private final T defaultValue;

    /** Current value; guarded to the game thread by convention. */
    private T value;

    protected Setting(String id, String displayName, String description, T defaultValue) {
        this.id = Objects.requireNonNull(id, "id");
        this.displayName = displayName == null ? id : displayName;
        this.description = description == null ? "" : description;
        this.defaultValue = Objects.requireNonNull(defaultValue, "defaultValue");
        this.value = defaultValue;
    }

    public final String id() {
        return id;
    }

    public final String displayName() {
        return displayName;
    }

    public final String description() {
        return description;
    }

    public final T defaultValue() {
        return defaultValue;
    }

    public final T get() {
        return value;
    }

    /**
     * Set through validation; invalid values keep the previous one and are
     * reported via the returned boolean rather than throwing (config files
     * are untrusted input).
     */
    public final boolean trySet(Object raw) {
        T parsed = parse(raw);
        if (!isValid(parsed)) {
            return false;
        }
        this.value = parsed;
        return true;
    }

    public final void reset() {
        this.value = defaultValue;
    }

    /** Convert an incoming raw JSON value into {@code T}, or null if wrong type. */
    protected abstract T parse(Object raw);

    /** Range/shape check after parsing. Default: non-null. */
    protected boolean isValid(T value) {
        return value != null;
    }

    /** Type tag for config serialization + future UI rendering. */
    public abstract String typeTag();
}
