import { useEffect, useState } from "react";
import { getSystemInfo, type SystemInfo } from "./api";

// Milestone-1 shell: navigation skeleton only (spec §23). Real screens arrive
// with their milestones; do not grow this file into a god component.
type Section = "Home" | "Instances" | "Mods" | "Worlds" | "Servers" | "Settings";

const SECTIONS: readonly Section[] = [
  "Home",
  "Instances",
  "Mods",
  "Worlds",
  "Servers",
  "Settings",
] as const;

export default function App() {
  const [section, setSection] = useState<Section>("Home");
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSystemInfo()
      .then(setInfo)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">ISEKAIYO</div>
        <nav>
          {SECTIONS.map((s) => (
            <button
              key={s}
              className={s === section ? "active" : ""}
              onClick={() => setSection(s)}
            >
              {s}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">
        <h1>{section}</h1>
        {error && <p className="error">Backend unreachable: {error}</p>}
        {info && !error && (
          <p className="muted">
            core v{info.app_version} · {info.target} · {info.profile} build
          </p>
        )}
        {!info && !error && <p className="muted">Connecting to core…</p>}
      </main>
    </div>
  );
}
