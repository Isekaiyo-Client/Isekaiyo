package net.isekaiyo.client.core.settings;

import java.util.Objects;

/**
 * Concrete setting types. Constructed via the static factories so the
 * generic plumbing stays in one place:
 *
 * <pre>{@code
 *   private final Setting<Boolean> enabled = StandardSettings.bool("enabled", true);
 *   private final Setting<Float>   opacity = StandardSettings.floating("opacity", 1.0f, 0.1f, 1.0f);
 * }</pre>
 */
public final class StandardSettings {

    private StandardSettings() {}

    public static BooleanSetting bool(String id, boolean defaultValue) {
        return new BooleanSetting(id, defaultValue);
    }

    /** Integer with inclusive bounds. */
    public static IntSetting integer(String id, int defaultValue, int min, int max) {
        return new IntSetting(id, defaultValue, min, max);
    }

    /** Float with inclusive bounds. */
    public static FloatSetting floating(String id, float defaultValue, float min, float max) {
        return new FloatSetting(id, defaultValue, min, max);
    }

    public static StringSetting text(String id, String defaultValue, int maxLength) {
        return new StringSetting(id, defaultValue, maxLength);
    }

    /** Enum-valued setting; the enum itself is the type, not a string. */
    public static <E extends Enum<E>> EnumSetting<E> enumerated(
            String id, Class<E> type, E defaultValue) {
        return new EnumSetting<>(id, type, defaultValue);
    }

    // ------------------------------------------------------------------

    public static final class BooleanSetting extends Setting<Boolean> {
        BooleanSetting(String id, boolean def) {
            super(id, id, "", def);
        }

        @Override
        protected Boolean parse(Object raw) {
            if (raw instanceof Boolean b) {
                return b;
            }
            // Gson sometimes delivers 0/1 from hand-edited files.
            if (raw instanceof Number n && (n.intValue() == 0 || n.intValue() == 1)) {
                return n.intValue() == 1;
            }
            return null;
        }

        @Override
        public String typeTag() {
            return "boolean";
        }
    }

    public static final class IntSetting extends Setting<Integer> {
        private final int min;
        private final int max;

        IntSetting(String id, int def, int min, int max) {
            super(id, id, "", def);
            this.min = min;
            this.max = max;
        }

        public int min() {
            return min;
        }

        public int max() {
            return max;
        }

        @Override
        protected Integer parse(Object raw) {
            return raw instanceof Number n ? n.intValue() : null;
        }

        @Override
        protected boolean isValid(Integer value) {
            return value != null && value >= min && value <= max;
        }

        @Override
        public String typeTag() {
            return "int";
        }
    }

    public static final class FloatSetting extends Setting<Float> {
        private final float min;
        private final float max;

        FloatSetting(String id, float def, float min, float max) {
            super(id, id, "", def);
            this.min = min;
            this.max = max;
        }

        public float min() {
            return min;
        }

        public float max() {
            return max;
        }

        @Override
        protected Float parse(Object raw) {
            return raw instanceof Number n ? n.floatValue() : null;
        }

        @Override
        protected boolean isValid(Float value) {
            return value != null && Float.isFinite(value) && value >= min && value <= max;
        }

        @Override
        public String typeTag() {
            return "float";
        }
    }

    public static final class StringSetting extends Setting<String> {
        private final int maxLength;

        StringSetting(String id, String def, int maxLength) {
            super(id, id, "", def);
            this.maxLength = maxLength;
        }

        @Override
        protected String parse(Object raw) {
            return raw instanceof String s ? s : null;
        }

        @Override
        protected boolean isValid(String value) {
            return value != null && value.length() <= maxLength;
        }

        @Override
        public String typeTag() {
            return "string";
        }
    }

    public static final class EnumSetting<E extends Enum<E>> extends Setting<E> {
        private final Class<E> type;

        EnumSetting(String id, Class<E> type, E def) {
            super(id, id, "", Objects.requireNonNull(def));
            this.type = type;
        }

        public Class<E> type() {
            return type;
        }

        @Override
        @SuppressWarnings("unchecked")
        protected E parse(Object raw) {
            if (raw instanceof String s) {
                try {
                    return Enum.valueOf(type, s);
                } catch (IllegalArgumentException ignored) {
                    // Unknown enum constant: treated as invalid, not fatal.
                    return null;
                }
            }
            return null;
        }

        @Override
        public String typeTag() {
            return "enum:" + type.getSimpleName();
        }
    }
}
