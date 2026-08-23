package net.isekaiyo.client.core.lifecycle;

/** Explicit client lifecycle — never random booleans (spec §3). */
public enum LifecycleState {
    BOOTSTRAPPING,
    INITIALIZING,
    READY,
    SHUTTING_DOWN,
    /** Terminal state after a failed init; the adapter must not tick us. */
    FAILED;

    public boolean isOperational() {
        return this == READY;
    }

    /** Ordered transitions only; anything else is a programming error upstream. */
    public boolean canTransitionTo(LifecycleState next) {
        return switch (this) {
            case BOOTSTRAPPING -> next == INITIALIZING || next == FAILED;
            case INITIALIZING -> next == READY || next == FAILED;
            case READY -> next == SHUTTING_DOWN;
            case SHUTTING_DOWN -> false; // terminal
            case FAILED -> false; // terminal
        };
    }
}
