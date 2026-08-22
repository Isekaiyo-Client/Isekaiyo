// Placeholder for sections that do not exist yet (spec §2): the UI must
// accurately represent the current state of the software.
import type { ReactNode } from "react";

const DESCRIPTIONS: Record<string, ReactNode> = {
  Mods:
    "Mod installation and dependency resolution are planned for the mod-management milestone. " +
    "Nothing here is functional yet.",
  Marketplace:
    "The marketplace integrates official provider APIs (Modrinth first). Integration begins after " +
    "instance and version foundations land. Nothing here is functional yet.",
  Client:
    "Isekaiyo's first-party client (HUD modules, PvP tools, performance features) is its own major " +
    "workstream — see docs/client-architecture.md. Nothing here is functional yet.",
};

export function Placeholder({ section }: { section: string }) {
  const description = DESCRIPTIONS[section] ?? "This area has not been built yet.";
  return (
    <section className="view" aria-label={section}>
      <header className="view-head">
        <h1>{section}</h1>
        <p className="muted">Under development</p>
      </header>
      <div className="panel">
        <div className="empty-state compact">
          <div className="empty-glyph" aria-hidden="true">
            ○
          </div>
          <p>{description}</p>
        </div>
      </div>
    </section>
  );
}
