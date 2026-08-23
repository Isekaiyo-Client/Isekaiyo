// Home — the launcher dashboard. Play runs the REAL pipeline: install (if
// needed) → Java resolution → launch plan → Minecraft process, with live
// phase/status polling and a console view of actual game output.
// Identity comes from the ACTIVE ACCOUNT (Phase 9); launching without one
// explains exactly what's missing — it never guesses an arbitrary identity.
import { useCallback, useEffect, useRef, useState } from "react";
import {
  accountGetActive,
  installInstance,
  launchInstance,
  launchStatus,
  readLaunchLog,
  stopLaunch,
  toErrorMessage,
  type AccountDto,
  type Instance,
  type LaunchStatusDto,
} from "../api";
import { Banner, Button, Dialog, EmptyState, Spinner } from "../components/ui";

const PHASE_TEXT: Record<string, string> = {
  idle: "Idle",
  preparing: "Preparing…",
  "resolving-metadata": "Resolving Minecraft metadata…",
  downloading: "Downloading files…",
  verifying: "Verifying files…",
  "resolving-java": "Resolving Java runtime…",
  "building-plan": "Building launch plan…",
  starting: "Starting Minecraft…",
  running: "Minecraft is running",
  stopping: "Stopping Minecraft…",
  completed: "Minecraft exited normally",
  failed: "Minecraft crashed",
  cancelled: "Launch cancelled",
};

export function Home({
  instances,
  selected,
  onNavigate,
  onSelect,
}: {
  instances: Instance[];
  selected: string | null;
  onNavigate: (section: "instances" | "settings" | "accounts") => void;
  onSelect: (id: string | null) => void;
}) {
  const [activeAccount, setActiveAccount] = useState<AccountDto | null>(null);
  const [working, setWorking] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ kind: "warn" | "error" | "info"; text: string } | null>(
    null,
  );
  const [status, setStatus] = useState<LaunchStatusDto | null>(null);
  const [logText, setLogText] = useState<string>("");
  const [showConsole, setShowConsole] = useState(false);
  const pollRef = useRef<number | null>(null);

  const active = instances.find((i) => i.id === selected) ?? null;

  // The active account is resolved fresh so a switch in Accounts reflects here.
  useEffect(() => {
    accountGetActive()
      .then(setActiveAccount)
      .catch(() => setActiveAccount(null));
  }, []);

  // Poll launch status while a launch is in flight or the game runs.
  const poll = useCallback(async () => {
    try {
      const s = await launchStatus();
      setStatus(s);
      if (s.phase !== "running" && s.phase !== "idle" && s.exit === null) {
        // keep polling through the pipeline phases
      } else if (pollRef.current !== null && (s.exit !== null || s.phase === "idle")) {
        window.clearInterval(pollRef.current);
        pollRef.current = null;
        if (s.exit) {
          setNotice({
            kind: s.exit.category === "completed" ? "info" : "error",
            text:
              s.exit.category === "completed"
                ? "Minecraft exited normally."
                : s.exit.category === "user-stopped"
                  ? "Minecraft was stopped."
                  : `Minecraft crashed (exit code ${s.exit.exit_code ?? "signal"}). Check the console for details.`,
          });
        }
      }
    } catch {
      /* status polling is best-effort */
    }
  }, []);

  useEffect(() => {
    void poll(); // pick up any already-running game after a restart of the UI
    return () => {
      if (pollRef.current !== null) window.clearInterval(pollRef.current);
    };
  }, [poll]);

  async function launch() {
    if (!active) return;
    setNotice(null);
    setWorking("Preparing…");
    try {
      // Install first (cheap when everything is already present).
      const report = await installInstance(active.id);
      if (report.failed.length > 0) {
        setNotice({
          kind: "error",
          text: `Installation failed: ${report.failed[0]}`,
        });
        return;
      }
      setWorking("Starting Minecraft…");
      const pid = await launchInstance(active.id);
      setNotice({ kind: "info", text: `Minecraft starting (PID ${pid}).` });
      if (pollRef.current !== null) window.clearInterval(pollRef.current);
      pollRef.current = window.setInterval(() => void poll(), 2000);
    } catch (e) {
      setNotice({ kind: "error", text: toErrorMessage(e) });
    } finally {
      setWorking(null);
    }
  }

  function requestPlay() {
    if (!activeAccount) {
      setNotice({
        kind: "warn",
        text:
          "No account connected. Add a Microsoft account or create an offline profile in Accounts first.",
      });
      return;
    }
    void launch();
  }

  async function openConsole() {
    setShowConsole(true);
    try {
      setLogText(await readLaunchLog());
    } catch {
      setLogText("(log unavailable)");
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
                {active.loader.version ? ` ${active.loader.version}` : ""}
              </span>
              <span className="muted play-note">
                {activeAccount
                  ? `Playing as ${activeAccount.username} (${activeAccount.kind === "microsoft" ? "Microsoft" : "Offline"}).`
                  : "No account connected — add one in Accounts."}
              </span>
            </div>
            {notice && <Banner kind={notice.kind}>{notice.text}</Banner>}
            {status && status.phase !== "idle" && (
              <div className="status-line" role="status">
                {PHASE_TEXT[status.phase] ?? status.phase}
                {status.phase === "running" && status.pid ? ` (PID ${status.pid})` : ""}
              </div>
            )}
            <div className="play-actions">
              <Button variant="ghost" onClick={() => onSelect(null)} ariaLabel="Clear selected instance">
                Clear selection
              </Button>
              <Button
                variant="ghost"
                onClick={() => void openConsole()}
                disabled={!status || status.phase === "idle"}
                ariaLabel="Open game console"
              >
                Console
              </Button>
              {status?.phase === "running" ? (
                <Button
                  variant="danger"
                  onClick={() => void stopLaunch()}
                  ariaLabel={`Stop ${active.name}`}
                >
                  ■ Stop
                </Button>
              ) : (
                <Button variant="primary" disabled={working !== null} onClick={requestPlay} ariaLabel={`Play ${active.name}`}>
                  {working ?? "▶ Play"}
                </Button>
              )}
            </div>
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
          <p className="muted">Launch history lands in the next iteration.</p>
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

      {showConsole && (
        <Dialog title="Game console" onClose={() => setShowConsole(false)}>
          <div className="dialog-body">
            <pre className="console" aria-label="Minecraft output">
              {logText || <Spinner label="Waiting for output…" />}
            </pre>
            <div className="dialog-actions">
              <Button variant="ghost" onClick={() => void openConsole()}>
                Refresh
              </Button>
              <Button
                variant="ghost"
                onClick={() => {
                  void navigator.clipboard?.writeText(logText);
                }}
              >
                Copy
              </Button>
              <Button variant="primary" onClick={() => setShowConsole(false)}>
                Close
              </Button>
            </div>
          </div>
        </Dialog>
      )}
    </section>
  );
}
