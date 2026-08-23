// App — the application container. Owns data flow (config + instances) and
// navigation; views render and call back (spec §6/§19). Business logic lives
// behind the typed API in api.ts, never in views.
import { useCallback, useEffect, useState } from "react";
import {
  getConfig,
  getStartupInfo,
  getSystemInfo,
  listInstances,
  setConfig,
  toErrorMessage,
  type AppConfig,
  type ConfigLoadInfo,
  type InstanceListing,
  type SystemInfo,
} from "./api";
import { Banner } from "./components/ui";
import { About } from "./views/About";
import { Accounts } from "./views/Accounts";
import { Home } from "./views/Home";
import { Instances } from "./views/Instances";
import { Mods } from "./views/Mods";
import { Placeholder } from "./views/Placeholder";
import { Settings } from "./views/Settings";

type Section =
  | "home"
  | "instances"
  | "mods"
  | "marketplace"
  | "client"
  | "accounts"
  | "settings"
  | "about";

const NAV: readonly { id: Section; label: string; soon?: boolean }[] = [
  { id: "home", label: "Home" },
  { id: "instances", label: "Instances" },
  { id: "mods", label: "Mods" },
  { id: "marketplace", label: "Marketplace", soon: true },
  { id: "client", label: "Client", soon: true },
  { id: "accounts", label: "Accounts" },
  { id: "settings", label: "Settings" },
  { id: "about", label: "About" },
];

export default function App() {
  const [section, setSection] = useState<Section>("home");
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [config, setConfigState] = useState<AppConfig | null>(null);
  const [listing, setListing] = useState<InstanceListing | null>(null);
  const [startup, setStartup] = useState<ConfigLoadInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshInstances = useCallback(() => {
    listInstances()
      .then(setListing)
      .catch((e) => setError(toErrorMessage(e)));
  }, []);

  useEffect(() => {
    void (async () => {
      try {
        const [sys, cfg, startupInfo] = await Promise.all([
          getSystemInfo(),
          getConfig(),
          getStartupInfo().catch(() => null), // optional nicety; never blocks startup
        ]);
        setInfo(sys);
        setConfigState(cfg);
        if (startupInfo) setStartup(startupInfo);
        setSection(cfg.start_page === "instances" ? "instances" : "home");
        refreshInstances();
      } catch (e) {
        setError(toErrorMessage(e));
      } finally {
        setLoading(false);
      }
    })();
  }, [refreshInstances]);

  async function patchConfig(patch: Partial<AppConfig>) {
    if (!config) return;
    const next = { ...config, ...patch };
    setConfigState(next); // optimistic; core is the source of truth on failure
    try {
      const saved = await setConfig(next);
      setConfigState(saved);
    } catch (e) {
      setError(toErrorMessage(e));
    }
  }

  function navigate(to: Section) {
    setSection(to);
  }

  const startupWarning =
    startup?.source === "recoveredcorrupt"
      ? `Your configuration file was malformed and has been reset to defaults. The original file was preserved at: ${startup.corrupt_backup_path ?? "(unknown location)"}`
      : null;

  if (!config && loading) {
    return (
      <div className="boot">
        <div className="brand">ISEKAIYO</div>
        <p className="muted">Starting…</p>
      </div>
    );
  }

  return (
    <div
      className={`shell theme-${config?.theme ?? "amoled"}${config?.animations_enabled === false ? " no-anim" : ""}`}
    >
      <aside className="sidebar">
        <div className="brand">ISEKAIYO</div>
        <nav aria-label="Main">
          {NAV.map((item) => (
            <button
              key={item.id}
              type="button"
              className={section === item.id ? "active" : ""}
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => navigate(item.id)}
            >
              <span>{item.label}</span>
              {item.soon && (
                <abbr className="soon" title="Under development">
                  soon
                </abbr>
              )}
            </button>
          ))}
        </nav>
        {info && (
          <footer className="sidebar-foot muted">
            v{info.app_version} · {info.profile}
          </footer>
        )}
      </aside>

      <main className="content">
        {error && (
          <Banner kind="error">
            {error}{" "}
            <button type="button" className="linkish" onClick={() => setError(null)}>
              dismiss
            </button>
          </Banner>
        )}
        {section === "home" && config && (
          <Home
            instances={listing?.instances ?? []}
            selected={config.selected_instance}
            onNavigate={(s) => navigate(s)}
            onSelect={(id) => void patchConfig({ selected_instance: id })}
          />
        )}
        {section === "instances" && config && (
          <Instances
            listing={listing}
            loading={loading}
            error={null}
            config={config}
            onSelectedChange={(id) => void patchConfig({ selected_instance: id })}
            onRefresh={refreshInstances}
          />
        )}
        {section === "mods" && config && (
          <Mods
            instance={
              listing?.instances.find((i) => i.id === config.selected_instance) ?? null
            }
          />
        )}
        {section === "accounts" && <Accounts />}
        {(section === "marketplace" || section === "client") && (
          <Placeholder section={section.charAt(0).toUpperCase() + section.slice(1)} />
        )}
        {section === "about" && <About info={info} />}
        {section === "settings" && config && (
          <Settings
            config={config}
            onChange={(patch) => void patchConfig(patch)}
            startupWarning={startupWarning}
          />
        )}
      </main>
    </div>
  );
}
