// Settings — persists through setConfig; a change here survives restart (the
// persistence proof is part of Milestone 001 acceptance).
import type { AppConfig, StartPage, SystemInfo, Theme } from "../api";
import { Banner } from "../components/ui";

const THEMES: readonly { value: Theme; label: string; hint: string }[] = [
  { value: "amoled", label: "AMOLED", hint: "True black, minimal noise" },
  { value: "modern", label: "Modern", hint: "Neutral dark surfaces" },
  { value: "sakura", label: "Sakura", hint: "Black with pink accents" },
];

export function Settings({
  config,
  onChange,
  info,
  startupWarning,
}: {
  config: AppConfig;
  onChange: (patch: Partial<AppConfig>) => void;
  info: SystemInfo | null;
  startupWarning: string | null;
}) {
  return (
    <section className="view" aria-label="Settings">
      <header className="view-head">
        <h1>Settings</h1>
        <p className="muted">Changes are saved immediately and persist across restarts.</p>
      </header>

      {startupWarning && <Banner kind="warn">{startupWarning}</Banner>}

      <div className="panel">
        <h2>Theme</h2>
        <div className="theme-grid" role="radiogroup" aria-label="Theme">
          {THEMES.map((t) => (
            <button
              key={t.value}
              type="button"
              role="radio"
              aria-checked={config.theme === t.value}
              className={`theme-card theme-${t.value}${config.theme === t.value ? " active" : ""}`}
              onClick={() => onChange({ theme: t.value })}
            >
              <span className="theme-swatch" aria-hidden="true" />
              <span className="theme-name">{t.label}</span>
              <span className="muted">{t.hint}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="panel">
        <h2>Startup</h2>
        <label className="field">
          <span className="field-label">Open on</span>
          <select
            value={config.start_page}
            onChange={(e) => onChange({ start_page: e.target.value as StartPage })}
          >
            <option value="home">Home</option>
            <option value="instances">Instances</option>
          </select>
        </label>
      </div>

      <div className="panel">
        <h2>About this build</h2>
        {info ? (
          <dl className="about-list">
            <dt>Core version</dt>
            <dd>{info.app_version}</dd>
            <dt>Platform</dt>
            <dd>{info.target}</dd>
            <dt>Profile</dt>
            <dd>{info.profile}</dd>
          </dl>
        ) : (
          <p className="muted">Connecting to core…</p>
        )}
        <p className="muted about-note">
          Isekaiyo is an independent open-source project, not affiliated with Mojang Studios or Microsoft.
        </p>
      </div>
    </section>
  );
}
