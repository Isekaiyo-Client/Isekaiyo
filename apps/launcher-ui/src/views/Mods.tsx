// Mods — per-instance mod management (Phase 6).
// Tabs: Installed (inventory + profiles) · Browse (Modrinth search).
// Every action goes through api.ts; no direct invoke here. Install shows the
// resolver's plan BEFORE downloading (§30/§31) and never fakes success.
import { useCallback, useEffect, useRef, useState } from "react";
import {
  modsCompatibleVersions,
  modsCreateProfile,
  modsInstall,
  modsInstallPlan,
  modsInventory,
  modsListProfiles,
  modsRemove,
  modsSearch,
  modsSetEnabled,
  modsSwitchProfile,
  modsUpdates,
  toErrorMessage,
  type Instance,
  type InstalledModDto,
  type ModInstallPlanDto,
  type ModInventoryDto,
  type ModProfileDto,
  type ModProjectDto,
  type ModUpdateDto,
} from "../api";
import { Banner, ConfirmDialog } from "../components/ui";

type Tab = "installed" | "browse";

const SORTS = [
  { value: "relevance", label: "Relevance" },
  { value: "downloads", label: "Downloads" },
  { value: "updated", label: "Updated" },
] as const;

function fmtDownloads(n: number): string {
  return n >= 1_000_000
    ? `${(n / 1_000_000).toFixed(1)}M`
    : n >= 1_000
      ? `${(n / 1_000).toFixed(0)}k`
      : String(n);
}

interface ModsProps {
  instance: Instance | null;
}

export function Mods({ instance }: ModsProps) {
  const [tab, setTab] = useState<Tab>("installed");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  if (!instance) {
    return (
      <section className="page">
        <h1>Mods</h1>
        <Banner kind="info">Select an instance first — mods are managed per instance.</Banner>
      </section>
    );
  }

  return (
    <section className="page" data-instance={instance.id}>
      <header className="page-head">
        <h1>
          Mods <span className="muted">· {instance.name} · MC {instance.minecraft_version} · {instance.loader.kind}</span>
        </h1>
        <div className="tabs" role="tablist">
          {(["installed", "browse"] as const).map((t) => (
            <button
              key={t}
              role="tab"
              aria-selected={tab === t}
              className={tab === t ? "active" : ""}
              onClick={() => setTab(t)}
            >
              {t === "installed" ? "Installed" : "Browse"}
            </button>
          ))}
        </div>
      </header>

      {error && (
        <Banner kind="error">
          {error}{" "}
          <button type="button" className="linkish" onClick={() => setError(null)}>
            dismiss
          </button>
        </Banner>
      )}
      {notice && (
        <Banner kind="info">
          {notice}{" "}
          <button type="button" className="linkish" onClick={() => setNotice(null)}>
            dismiss
          </button>
        </Banner>
      )}

      {tab === "installed" ? (
        <InstalledTab instanceId={instance.id} onError={setError} onNotice={setNotice} />
      ) : (
        <BrowseTab
          instanceId={instance.id}
          onError={setError}
          onNotice={setNotice}
          onChanged={() => setTab("installed")}
        />
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------

function InstalledTab({
  instanceId,
  onError,
  onNotice,
}: {
  instanceId: string;
  onError: (m: string) => void;
  onNotice: (m: string) => void;
}) {
  const [inv, setInv] = useState<ModInventoryDto | null>(null);
  const [profiles, setProfiles] = useState<ModProfileDto[]>([]);
  const [updates, setUpdates] = useState<Record<string, ModUpdateDto>>({});
  const [loading, setLoading] = useState(true);
  const [confirmRemove, setConfirmRemove] = useState<InstalledModDto | null>(null);
  // Armed after the backend refuses a removal due to reverse dependencies;
  // the next confirm sends force=true (the user has seen the explanation).
  const [forceArmed, setForceArmed] = useState(false);
  const [newProfileName, setNewProfileName] = useState("");

  const refresh = useCallback(() => {
    setLoading(true);
    Promise.all([modsInventory(instanceId), modsListProfiles(instanceId)])
      .then(([i, p]) => {
        setInv(i);
        setProfiles(p);
      })
      .catch((e) => onError(toErrorMessage(e)))
      .finally(() => setLoading(false));
  }, [instanceId, onError]);

  useEffect(refresh, [refresh]);

  function checkUpdates() {
    modsUpdates(instanceId)
      .then((list) => {
        setUpdates(Object.fromEntries(list.map((u) => [u.project_id, u])));
        if (list.every((u) => u.state !== "update-available")) {
          onNotice("All managed mods are up to date.");
        }
      })
      .catch((e) => onError(toErrorMessage(e)));
  }

  function toggle(mod: InstalledModDto) {
    if (!mod.project_id) return;
    modsSetEnabled(instanceId, mod.project_id, !mod.enabled)
      .then(refresh)
      .catch((e) => onError(toErrorMessage(e)));
  }

  function doRemove(force: boolean) {
    const mod = confirmRemove;
    if (!mod?.project_id) return;
    modsRemove(instanceId, mod.project_id, force)
      .then(() => {
        setConfirmRemove(null);
        setForceArmed(false);
        onNotice(`${mod.title} removed.`);
        refresh();
      })
      .catch((e) => {
        const msg = toErrorMessage(e);
        if (msg.includes("still required by")) {
          // Refusal → arm force so the next confirm removes it anyway.
          setForceArmed(true);
        } else {
          setConfirmRemove(null);
          onError(msg);
        }
      });
  }

  function createProfile() {
    const name = newProfileName.trim();
    if (!name) return;
    modsCreateProfile(instanceId, name)
      .then((p) => {
        setProfiles(p);
        setNewProfileName("");
        onNotice(`Profile “${name}” snapshotted the current enabled set.`);
      })
      .catch((e) => onError(toErrorMessage(e)));
  }

  function switchTo(profileId: string | null) {
    modsSwitchProfile(instanceId, profileId)
      .then(refresh)
      .catch((e) => onError(toErrorMessage(e)));
  }

  return (
    <>
      <div className="toolbar">
        <button type="button" className="btn btn-secondary" onClick={checkUpdates}>
          Check for updates
        </button>
        <button type="button" className="btn btn-secondary" onClick={refresh}>
          Refresh
        </button>
      </div>

      {loading && <p className="muted">Loading installed mods…</p>}
      {!loading && inv && inv.mods.length === 0 && (
        <Banner kind="info">
          No mods installed. Switch to <strong>Browse</strong> to install some.
        </Banner>
      )}

      {inv && inv.mods.length > 0 && (
        <ul className="mod-list">
          {inv.mods.map((mod) => {
            const upd = mod.project_id ? updates[mod.project_id] : undefined;
            return (
              <li key={`${mod.state}:${mod.filename}`} className={`mod-row state-${mod.state}`}>
                <div className="mod-info">
                  <span className="mod-title">{mod.title}</span>
                  <span className="muted mono">{mod.filename}</span>
                  <span className={`badge ${mod.state}`}>{mod.state.toUpperCase()}</span>
                  {!mod.enabled && mod.state === "managed" && (
                    <span className="badge disabled">DISABLED</span>
                  )}
                  {upd?.state === "update-available" && (
                    <span className="badge update">
                      UPDATE → {upd.available_version ?? "?"}
                    </span>
                  )}
                  {upd?.state === "incompatible" && (
                    <span className="badge warn">INCOMPATIBLE NOW</span>
                  )}
                  {mod.warning && <span className="muted small">{mod.warning}</span>}
                </div>
                {mod.project_id && (
                  <div className="mod-actions">
                    {mod.state === "managed" && (
                      <button type="button" className="btn btn-secondary" onClick={() => toggle(mod)}>
                        {mod.enabled ? "Disable" : "Enable"}
                      </button>
                    )}
                    <button
                      type="button"
                      className="btn btn-danger"
                      onClick={() => setConfirmRemove(mod)}
                    >
                      Remove
                    </button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <h2 className="section-title">Profiles</h2>
      <p className="muted small">
        A profile snapshots which mods are enabled. Switching renames files only — nothing is
        re-downloaded.
      </p>
      <div className="toolbar">
        <input
          type="text"
          placeholder="New profile name"
          value={newProfileName}
          onChange={(e) => setNewProfileName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && createProfile()}
        /><button type="button" className="btn btn-primary" onClick={createProfile} disabled={!newProfileName.trim()}>
          Snapshot current set
        </button>
        <button type="button" className="btn btn-secondary" onClick={() => switchTo(null)}>
          Enable all (no profile)
        </button>
      </div>
      {profiles.length > 0 && (
        <ul className="profile-list">
          {profiles.map((p) => (
            <li key={p.id} className={p.active ? "active" : ""}>
              <span>{p.name}</span>
              <span className="muted">{p.enabled_count} enabled</span>
              {!p.active && (
                <button type="button" className="btn btn-secondary" onClick={() => switchTo(p.id)}>
                  Activate
                </button>
              )}
              {p.active && <span className="badge active">ACTIVE</span>}
            </li>
          ))}
        </ul>
      )}

      {confirmRemove && (
        <ConfirmDialog
          title={forceArmed ? `Force remove ${confirmRemove.title}?` : `Remove ${confirmRemove.title}?`}
          body={
            forceArmed
              ? "Other installed mods depend on this one and will break until their dependency is reinstalled. Remove it anyway?"
              : "The mod file will be deleted from this instance's mods folder. Dependencies used by other mods are kept."
          }
          confirmLabel={forceArmed ? "Force remove" : "Remove"}
          onCancel={() => {
            setConfirmRemove(null);
            setForceArmed(false);
          }}
          onConfirm={() => doRemove(forceArmed)}
        />
      )}
    </>
  );
}

// ---------------------------------------------------------------------------

function BrowseTab({
  instanceId,
  onError,
  onNotice,
  onChanged,
}: {
  instanceId: string;
  onError: (m: string) => void;
  onNotice: (m: string) => void;
  onChanged: () => void;
}) {
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<string>("relevance");
  const [results, setResults] = useState<ModProjectDto[] | null>(null);
  const [searching, setSearching] = useState(false);
  const [planFor, setPlanFor] = useState<ModProjectDto | null>(null);
  const debounceRef = useRef<number | null>(null);
  // Guard against stale responses overwriting newer ones (§39).
  const requestSeq = useRef(0);

  const runSearch = useCallback(
    (q: string, sortKey: string) => {
      const seq = ++requestSeq.current;
      setSearching(true);
      modsSearch(instanceId, q, sortKey, 1)
        .then((hits) => {
          if (seq === requestSeq.current) setResults(hits);
        })
        .catch((e) => {
          if (seq === requestSeq.current) onError(toErrorMessage(e));
        })
        .finally(() => {
          if (seq === requestSeq.current) setSearching(false);
        });
    },
    [instanceId, onError],
  );

  // Debounced live search (§38).
  useEffect(() => {
    if (debounceRef.current !== null) window.clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => runSearch(query, sort), 350);
    return () => {
      if (debounceRef.current !== null) window.clearTimeout(debounceRef.current);
    };
  }, [query, sort, runSearch]);

  return (
    <>
      <div className="toolbar">
        <input
          type="search"
          placeholder="Search Modrinth (scoped to this instance's version + loader)"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Search mods"
        />
        <select value={sort} onChange={(e) => setSort(e.target.value)} aria-label="Sort order">
          {SORTS.map((s) => (
            <option key={s.value} value={s.value}>
              Sort: {s.label}
            </option>
          ))}
        </select>
      </div>

      {searching && <p className="muted">Searching…</p>}
      {!searching && results && results.length === 0 && (
        <Banner kind="info">No compatible mods matched.</Banner>
      )}

      {results && results.length > 0 && (
        <ul className="mod-grid">
          {results.map((r) => (
            <li key={r.project_id} className="mod-card">
              <div className="mod-card-head">
                <strong>{r.title}</strong>
                <span className="muted small">{fmtDownloads(r.downloads)} ↓</span>
              </div>
              <p className="small">{r.description || "(no description)"}</p>
              <div className="mod-meta muted small">
                {r.authors.length > 0 && <span>{r.authors.join(", ")}</span>}
                <span className="badge source">{r.source}</span>
              </div>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => setPlanFor(r)}
              >
                Install…
              </button>
            </li>
          ))}
        </ul>
      )}

      {planFor && (
        <InstallDialog
          instanceId={instanceId}
          project={planFor}
          onClose={() => setPlanFor(null)}
          onInstalled={(msg) => {
            setPlanFor(null);
            onNotice(msg);
            onChanged();
          }}
          onError={onError}
        />
      )}
    </>
  );
}

// ---------------------------------------------------------------------------

function InstallDialog({
  instanceId,
  project,
  onClose,
  onInstalled,
  onError,
}: {
  instanceId: string;
  project: ModProjectDto;
  onClose: () => void;
  onInstalled: (message: string) => void;
  onError: (m: string) => void;
}) {
  const [phase, setPhase] = useState<"planning" | "confirm" | "installing">("planning");
  const [plan, setPlan] = useState<ModInstallPlanDto | null>(null);

  useEffect(() => {
    let cancelled = false;
    modsInstallPlan(instanceId, project.source, project.project_id)
      .then((p) => {
        if (!cancelled) {
          setPlan(p);
          setPhase("confirm");
        }
      })
      .catch((e) => {
        if (!cancelled) {
          onError(toErrorMessage(e));
          onClose();
        }
      });
    return () => {
      cancelled = true;
    };
  }, [instanceId, project, onClose, onError]);

  function doInstall() {
    setPhase("installing");
    modsInstall(instanceId, project.source, project.project_id)
      .then((report) => {
        if (report.ok()) {
          const parts = [
            `${report.downloaded.length} downloaded`,
            report.skipped.length > 0 ? `${report.skipped.length} already present` : null,
            report.unverified.length > 0
              ? `${report.unverified.length} without hash verification`
              : null,
          ].filter(Boolean);
          onInstalled(`Installed ${project.title}: ${parts.join(", ")}.`);
        } else {
          onError(`Installation failed: ${report.failed.join("; ")}`);
        }
      })
      .catch((e) => onError(toErrorMessage(e)));
  }

  return (
    <div className="dialog-backdrop" role="presentation">
      <div className="dialog" role="dialog" aria-modal="true" aria-label={`Install ${project.title}`}>
        <h3>Install {project.title}</h3>
        {phase === "planning" && <p className="muted">Resolving dependencies…</p>}
        {plan && phase === "confirm" && (
          <>
            {!plan.is_installable() && (
              <Banner kind="error">
                This mod cannot be installed:
                {plan.conflicts.length > 0 && <> conflicts with {plan.conflicts.join(", ")}.</>}
                {plan.unsatisfiable.length > 0 && (
                  <> missing dependencies: {plan.unsatisfiable.join("; ")}.</>
                )}
              </Banner>
            )}
            {plan.is_installable() && (
              <ul className="plan-list">
                {plan.to_install.map((m) => (
                  <li key={m.project_id}>
                    {m.game_versions.includes("*") || m.title === m.project_id
                      ? m.title
                      : `${m.project_id} (${m.title})`}
                  </li>
                ))}
              </ul>
            )}
            {plan.already_installed.length > 0 && (
              <p className="muted small">
                Already satisfied: {plan.already_installed.join(", ")}
              </p>
            )}
          </>
        )}
        {phase === "installing" && <p className="muted">Downloading &amp; verifying…</p>}
        <div className="dialog-actions">
          <button type="button" className="btn btn-secondary" onClick={onClose} disabled={phase === "installing"}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={!plan || !plan.is_installable() || phase !== "confirm"}
            onClick={doInstall}
          >
            {plan && plan.is_installable()
              ? plan.to_install.length > 1
                ? `Install ${plan.to_install.length} mods`
                : "Install"
              : "Unavailable"}
          </button>
        </div>
      </div>
    </div>
  );
}
