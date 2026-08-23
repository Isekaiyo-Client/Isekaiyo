package net.isekaiyo.client.core.events;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Consumer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * The client event bus (spec §10–§11, §45).
 *
 * <p>Design notes:</p>
 * <ul>
 *   <li>Strongly typed: one bus instance per event type, held by the owner
 *       ({@code Events}). No reflective classpath scanning, no string keys.</li>
 *   <li>Exception isolation: a throwing listener is logged WITH its owner's
 *       name and skipped — the rest of the listeners, and the game, keep
 *       running. Errors are never silently swallowed.</li>
 *   <li>Hot-path friendly: dispatch iterates an array snapshot; subscribing
 *       uses copy-on-write so tick/render dispatch allocates nothing.</li>
 * </ul>
 */
public final class EventBus<T> {

    private static final Logger LOG = LoggerFactory.getLogger("Isekaiyo/Event");

    private final String eventName;
    private final CopyOnWriteArrayList<Listener<T>> listeners = new CopyOnWriteArrayList<>();

    public EventBus(String eventName) {
        this.eventName = eventName;
    }

    /**
     * Subscribe {@code handler}, attributed to {@code owner} (a module id or
     * subsystem name) for logging and bulk removal on module disable.
     */
    public Subscription subscribe(String owner, Consumer<T> handler) {
        Listener<T> listener = new Listener<>(owner, handler);
        listeners.add(listener);
        return () -> unsubscribe(listener);
    }

    /** Remove everything belonging to one owner (module disable/unload). */
    public void unsubscribeOwner(String owner) {
        listeners.removeIf(l -> l.owner.equals(owner));
    }

    /** Dispatch to every live listener; each failure is isolated + logged. */
    public void dispatch(T event) {
        List<Listener<T>> snapshot = listeners;
        for (int i = 0; i < snapshot.size(); i++) {
            Listener<T> l = snapshot.get(i);
            try {
                l.handler.accept(event);
            } catch (Throwable t) {
                // spec §11: identify the responsible owner, never crash.
                LOG.error(
                        "Listener '{}' failed during {} — isolated, other listeners continue",
                        l.owner,
                        eventName,
                        t
                );
            }
        }
    }

    public int listenerCount() {
        return listeners.size();
    }

    private void unsubscribe(Listener<T> listener) {
        listeners.remove(listener);
    }

    private record Listener<O>(String owner, Consumer<O> handler) {}

    /** Cancellable subscription handle; idempotent close. */
    public interface Subscription extends AutoCloseable {
        @Override
        void close();
    }
}
