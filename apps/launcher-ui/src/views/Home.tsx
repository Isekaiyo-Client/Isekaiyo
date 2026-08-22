// Home — the launcher dashboard. The Play control is wired to the real
// `launch_instance` command; until the Minecraft runtime milestone lands the
// core answers with `runtime.unavailable`, which we surface honestly instead
// of faking progress.
import { useState } from "react";
import { launchInstance, toErrorMessage, type CommandError, type Instance } from "../api";
import { Banner, Button, EmptyState } from "../components/ui";

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
  const [launching, setLaunching] = useState(false);
  const [notice, setNotice] = useState<{ kind: "warn" | "error"; text: string } | null>(null);
  const active = instances.find((i) => i.id === selected) ?? null;

  async function play() {
    if (!active) return;
    setLaunching(true);
    setNotice(null);
    try {
      await launchInstance(active.id);
      // Unreachable today; when the runtime exists this is where session UI goes.
    } catch (e) {
      const code = (e as Partial<CommandError>)?.code;
      setNotice({ kind: code === "runtime.unavailable" ? "warn" : "error", text: toErrorMessage(e) });
    } finally {
      setLaunching(false);
    }
  }

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
            {notice && <Banner kind={notice.kind}>{notice.text}</Banner>}
            <div className="play-actions">
              <Button variant="ghost" onClick={() => onSelect(null)} ariaLabel="Clear selected instance">
                Clear selection
              </Button>
              <Button variant="primary" disabled={launching} onClick={() => void play()} ariaLabel={`Play ${active.name}`}>
                {launching ? "Preparing…" : "▶ Play"}
              </Button>
            </div>
            <p className="muted play-note">
              The Minecraft runtime is not implemented yet — pressing Play reports that instead of pretending.
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
