package net.isekaiyo.client.core.modules;

/**
 * Module categories. Data, not UI: the client screen derives its filter list
 * from {@link ModuleManager#presentCategories()}, never from a hard-coded
 * list (spec §29).
 */
public enum ModuleCategory {
    COMBAT,
    MOVEMENT,
    PLAYER,
    RENDER,
    WORLD,
    HUD,
    MISC,
    CLIENT,
    PERFORMANCE
}
