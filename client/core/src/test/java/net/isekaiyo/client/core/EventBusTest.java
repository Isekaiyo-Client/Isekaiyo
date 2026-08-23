package net.isekaiyo.client.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import net.isekaiyo.client.core.events.EventBus;
import org.junit.jupiter.api.Test;

class EventBusTest {

    record Ping(int n) {}

    @Test
    void listenersFireInSubscriptionOrder() {
        EventBus<Ping> bus = new EventBus<>("Ping");
        List<Integer> order = new ArrayList<>();
        bus.subscribe("a", p -> order.add(p.n()));
        bus.subscribe("b", p -> order.add(p.n() * 10));
        bus.dispatch(new Ping(1));
        assertEquals(List.of(1, 10), order);
    }

    @Test
    void throwingListenerIsIsolatedAndIdentified() {
        EventBus<Ping> bus = new EventBus<>("Ping");
        boolean[] reached = {false};
        bus.subscribe("bad-module", p -> {
            throw new IllegalStateException("module bug");
        });
        bus.subscribe("good-module", p -> reached[0] = true);

        bus.dispatch(new Ping(1)); // must NOT throw
        assertTrue(reached[0], "listeners after the failing one still run");
    }

    @Test
    void subscriptionHandleRemovesExactlyOneListener() {
        EventBus<Ping> bus = new EventBus<>("Ping");
        int[] count = {0};
        var sub = bus.subscribe("a", p -> count[0]++);
        bus.subscribe("b", p -> count[0]++);
        sub.close();
        sub.close(); // idempotent
        bus.dispatch(new Ping(1));
        assertEquals(1, count[0]);
    }

    @Test
    void unsubscribeOwnerClearsAllOfOneOwner() {
        EventBus<Ping> a = new EventBus<>("A");
        EventBus<Ping> b = new EventBus<>("B");
        int[] hits = {0};
        a.subscribe("m", p -> hits[0]++);
        b.subscribe("m", p -> hits[0]++);
        net.isekaiyo.client.core.events.Events.unsubscribeOwner("m");
        // Note: Events' static buses are separate instances; direct check here.
        a.unsubscribeOwner("m");
        b.unsubscribeOwner("m");
        a.dispatch(new Ping(1));
        b.dispatch(new Ping(1));
        assertEquals(0, hits[0]);
        assertEquals(0, a.listenerCount());
        assertEquals(0, b.listenerCount());
    }
}
