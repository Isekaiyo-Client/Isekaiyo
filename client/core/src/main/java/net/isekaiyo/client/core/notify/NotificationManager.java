package net.isekaiyo.client.core.notify;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;

/**
 * Lightweight in-game toasts (spec §26). Notifications are queued as data;
 * a single HUD element draws them — the manager itself never renders.
 * Unobtrusive by design: bounded queue, automatic expiry.
 */
public final class NotificationManager {

    /** Max simultaneous toasts; oldest are dropped first. */
    private static final int MAX_VISIBLE = 4;
    /** Milliseconds a toast stays on screen. */
    private static final long DEFAULT_TTL_MS = 2_500;

    public enum Kind {
        INFO,
        SUCCESS,
        WARNING,
        ERROR
    }

    public record Notification(String title, String message, Kind kind, long expiresAtMs) {}

    private final ArrayDeque<Notification> active = new ArrayDeque<>();
    private final java.util.function.LongSupplier clock;

    /** {@code clock} returns monotonic ms (injected for testability). */
    public NotificationManager(java.util.function.LongSupplier clock) {
        this.clock = clock;
    }

    public void push(Kind kind, String title, String message) {
        synchronized (active) {
            while (active.size() >= MAX_VISIBLE) {
                active.pollFirst();
            }
            active.addLast(new Notification(
                    title,
                    message == null ? "" : message,
                    kind,
                    clock.getAsLong() + DEFAULT_TTL_MS));
        }
    }

    public void moduleState(String moduleName, boolean enabled) {
        push(
                enabled ? Kind.INFO : Kind.INFO,
                moduleName,
                enabled ? "enabled" : "disabled");
    }

    /** Expire + snapshot current toasts (called once per frame by the drawer). */
    public List<Notification> visible() {
        long now = clock.getAsLong();
        List<Notification> out = new ArrayList<>();
        synchronized (active) {
            while (!active.isEmpty() && active.peekFirst().expiresAtMs() <= now) {
                active.pollFirst();
            }
            out.addAll(active);
        }
        return out;
    }

    public int size() {
        synchronized (active) {
            return active.size();
        }
    }

    public void clear() {
        synchronized (active) {
            active.clear();
        }
    }
}
