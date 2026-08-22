// Home — the first screen (spec §5). Honest by design: the Play control is
// real UI but disabled until launching exists in a later milestone, and it
// says so instead of pretending.
import type { Instance } from "../api";
import { Button, EmptyState } from "../components/ui";

export function Home({
  instances,
  selected,
  onNavigate,
  onSelect,
}: {
  instances: Instance[];
  selected: string | null;
  onNavigate: (section: "instances" | "settings") => void;
  onSelect: (id: string | null) => void;
}) {
  const active = instances.find((i) => i.id === selected) ?? null;

  return (
    <section className="view" aria-label="Home">
      <header className="view-head">
        <h1>Welcome back</h1>
        <p className="muted">Your Minecraft environment, one place.</p>
      </header>

      <div className="home-play panel">
        {active ? (
          <>
            <div className="play-target">
              <span className="play-name">{active.name}</span>
              <span className="muted">
                {active.minecraft_version} · {active.loader.kind}
              </span>
            </div>
            <div className="play-actions">
              <Button variant="ghost" onClick={() => onSelect(null)} ariaLabel="Clear selected instance">
                Clear selection
              </Button>
              <Button variant="primary" disabled ariaLabel="Play is not available yet">
                ▶ Play
              </Button>
            </div>
            <p className="muted play-note">
              Launching arrives with the version-metadata milestone — the button stays disabled until it can do
              something real.
            </p>
          </>
        ) : (
          <EmptyState
            title="No instance selected"
            hint="Create or choose an instance to get started."
            action={<Button variant="primary" onClick={() => onNavigate("instances")}>Go to Instances</Button>}
          />
        )}
      </div>

      <div className="home-grid">
        <div className="panel">
          <h2>Recent activity</h2>
          <p className="muted">No recent activity. Actions you take will appear here.</p>
        </div>
        <div className="panel">
          <h2>Quick access</h2>
          <div className="quick-links">
            <button type="button" className="quick-link" onClick={() => onNavigate("instances")}>
              Instances <span className="muted">{instances.length}</span>
            </button>
            <button type="button" className="quick-link" onClick={() => onNavigate("settings")}>
              Settings
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}
