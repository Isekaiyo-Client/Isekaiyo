// Settings — persisted user preferences (schema v2). Every change goes
// through patchConfig in App (optimistic; core is the source of truth on
// failure) and survives restart. Build/version info lives on the About page.
import type { AppConfig, StartPage, Theme } from "../api";
import { Banner, SettingRow, Switch } from "../components/ui";

const THEMES: readonly { value: Theme; label: string; hint: string }[] = [
  { value: "amoled", label: "AMOLED", hint: "True black, sakura accent — the Isekaiyo identity" },
  { value: "modern", label: "Modern", hint: "Neutral dark surfaces, calm accent" },
  { value: "sakura", label: "Sakura", hint: "Deep plum-black, stronger sakura" },
];

export function Settings({
  config,
  onChange,
  startupWarning,
}: {
  config: AppConfig;
  onChange: (patch: Partial<AppConfig>) => void;
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
        <h2>General</h2>
        <SettingRow
          name="Confirm before deleting"
          hint="Ask for confirmation when deleting an instance."
        >
          <Switch
            checked={config.confirm_before_delete}
            onChange={(next) => onChange({ confirm_before_delete: next })}
            label="Confirm before deleting an instance"
          />
        </SettingRow>
      </div>

      <div className="panel">
        <h2>Appearance</h2>
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
        <SettingRow name="Animations" hint="Disable to remove all motion from the interface.">
          <Switch
            checked={config.animations_enabled}
            onChange={(next) => onChange({ animations_enabled: next })}
            label="Interface animations"
          />
        </SettingRow>
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
    </section>
  );
}
