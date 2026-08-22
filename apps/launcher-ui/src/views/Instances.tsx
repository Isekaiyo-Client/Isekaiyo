// Instances — real instance management. Version choices come from the actual
// Mojang manifest via the backend; loader versions come from Fabric/Quilt meta
// services. Nothing here is hard-coded or faked.
import { useEffect, useMemo, useState } from "react";
import {
  createInstance,
  deleteInstance,
  installInstance,
  listLoaderVersions,
  listVersions,
  toErrorMessage,
  updateInstance,
  type AppConfig,
  type InstallReportDto,
  type Instance,
  type InstanceListing,
  type LoaderKind,
  type LoaderVersionDto,
  type VersionListDto,
} from "../api";
import {
  Banner,
  Button,
  ConfirmDialog,
  Dialog,
  EmptyState,
  Field,
  LoaderBadge,
  Spinner,
} from "../components/ui";
import {
  labelForLoader,
  loaderNeedsVersion,
  validateInstanceForm,
  type FormErrors,
  type InstanceForm,
} from "../lib/validate";

type DialogKind =
  | { kind: "none" }
  | { kind: "create" }
  | { kind: "edit"; target: Instance }
  | { kind: "delete"; target: Instance };

/** Loaders the engine can actually install today. */
const SUPPORTED_LOADERS: readonly LoaderKind[] = ["vanilla", "fabric", "quilt"];
const PLANNED_LOADERS: readonly LoaderKind[] = ["forge", "neoforge"];

function emptyForm(): InstanceForm {
  return { name: "", minecraftVersion: "", loaderKind: "vanilla", loaderVersion: "" };
}

function formOf(instance: Instance): InstanceForm {
  return {
    name: instance.name,
    minecraftVersion: instance.minecraft_version,
    loaderKind: instance.loader.kind,
    loaderVersion: instance.loader.version ?? "",
  };
}

function formatDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString();
}

export function Instances({
  listing,
  loading,
  error,
  config,
  onSelectedChange,
  onRefresh,
}: {
  listing: InstanceListing | null;
  loading: boolean;
  error: string | null;
  config: AppConfig;
  onSelectedChange: (id: string | null) => void;
  onRefresh: () => void;
}) {
  const [dialog, setDialog] = useState<DialogKind>({ kind: "none" });
  const [form, setForm] = useState<InstanceForm>(emptyForm());
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [installNote, setInstallNote] = useState<string | null>(null);

  // Real Minecraft version metadata.
  const [versions, setVersions] = useState<VersionListDto | null>(null);
  const [versionsError, setVersionsError] = useState<string | null>(null);
  const [versionQuery, setVersionQuery] = useState("");
  const [showSnapshots, setShowSnapshots] = useState(false);

  // Loader versions for the chosen (loader, mc version) pair.
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersionDto[]>([]);
  const [loaderVersionsState, setLoaderVersionsState] = useState<
    "idle" | "loading" | "error"
  >("idle");

  const instances = useMemo(() => listing?.instances ?? [], [listing]);

  // Fetch the manifest once when the create dialog opens.
  useEffect(() => {
    if (dialog.kind !== "create" || versions || versionsError) return;
    listVersions()
      .then((list) => {
        setVersions(list);
        if (!form.minecraftVersion) {
          const latestRelease = [...list.entries]
            .reverse()
            .find((e) => e.kind === "release");
          if (latestRelease) {
            setForm((f) => ({ ...f, minecraftVersion: latestRelease.id }));
          }
        }
      })
      .catch((e) => setVersionsError(toErrorMessage(e)));
  }, [dialog.kind]);

  // Fetch compatible loader versions whenever (loader, mc) changes.
  useEffect(() => {
    if (
      dialog.kind !== "create" ||
      !loaderNeedsVersion(form.loaderKind) ||
      !form.minecraftVersion
    ) {
      setLoaderVersions([]);
      return;
    }
    let cancelled = false;
    setLoaderVersionsState("loading");
    listLoaderVersions(form.loaderKind, form.minecraftVersion)
      .then((v) => {
        if (cancelled) return;
        setLoaderVersions(v.filter((x) => x.stable));
        setLoaderVersionsState("idle");
      })
      .catch((e) => {
        if (cancelled) return;
        setActionError(`${labelForLoader(form.loaderKind)}: ${toErrorMessage(e)}`);
        setLoaderVersions([]);
        setLoaderVersionsState("error");
      });
    return () => {
      cancelled = true;
    };
  }, [dialog.kind, form.loaderKind, form.minecraftVersion]);

  const filteredVersions = useMemo(() => {
    if (!versions) return [];
    const q = versionQuery.trim().toLowerCase();
    return versions.entries
      .filter((e) => (showSnapshots ? true : e.kind === "release"))
      .filter((e) => (q ? e.id.toLowerCase().includes(q) : true))
      .slice(0, 60); // keep the DOM sane; search narrows further
  }, [versions, versionQuery, showSnapshots]);

  async function submit() {
    const found = validateInstanceForm(form);
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    setBusy(true);
    setActionError(null);
    try {
      if (dialog.kind === "create") {
        await createInstance(
          form.name.trim(),
          form.minecraftVersion.trim(),
          loaderNeedsVersion(form.loaderKind)
            ? { kind: form.loaderKind, version: form.loaderVersion.trim() }
            : null,
        );
      } else if (dialog.kind === "edit") {
        await updateInstance({
          ...dialog.target,
          name: form.name.trim(),
          minecraft_version: form.minecraftVersion.trim(),
          loader: {
            kind: form.loaderKind,
            version: loaderNeedsVersion(form.loaderKind) ? form.loaderVersion.trim() : null,
          },
        });
      }
      setDialog({ kind: "none" });
      onRefresh();
    } catch (e) {
      setActionError(toErrorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function removeInstance(target: Instance) {
    setBusy(true);
    setActionError(null);
    try {
      await deleteInstance(target.id);
      if (config.selected_instance === target.id) {
        onSelectedChange(null);
      }
      setDialog({ kind: "none" });
      onRefresh();
    } catch (e) {
      setActionError(toErrorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  /** Honors Settings → General → “Confirm before deleting”. */
  function requestDelete(target: Instance) {
    if (config.confirm_before_delete) {
      setDialog({ kind: "delete", target });
    } else {
      void removeInstance(target);
    }
  }

  /** Download all artifacts an instance needs so Play is instant later. */
  async function prepare(target: Instance) {
    setBusy(true);
    setActionError(null);
    setInstallNote(null);
    try {
      const report: InstallReportDto = await installInstance(target.id);
      onRefresh();
      setInstallNote(
        report.failed.length > 0
          ? `Installed with ${report.failed.length} failure(s): ${report.failed[0]}`
          : `${target.name} ready — ${report.downloaded} downloaded, ${report.skipped} already present.`,
      );
    } catch (e) {
      setActionError(toErrorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  function openCreate() {
    setForm(emptyForm());
    setErrors({});
    setDialog({ kind: "create" });
  }

  function openEdit(target: Instance) {
    setForm(formOf(target));
    setErrors({});
    setDialog({ kind: "edit", target });
  }

  return (
    <section className="view" aria-label="Instances">
      <header className="view-head row">
        <div>
          <h1>Instances</h1>
          <p className="muted">Isolated Minecraft environments.</p>
        </div>
        <Button variant="primary" onClick={openCreate}>
          + Create Instance
        </Button>
      </header>

      {actionError && (
        <Banner kind="error">
          {actionError}{" "}
          <button type="button" className="linkish" onClick={() => setActionError(null)}>
            dismiss
          </button>
        </Banner>
      )}
      {installNote && (
        <Banner kind={installNote.includes("failure") ? "warn" : "info"}>
          {installNote}{" "}
          <button type="button" className="linkish" onClick={() => setInstallNote(null)}>
            dismiss
          </button>
        </Banner>
      )}
      {listing && listing.unreadable_files > 0 && (
        <Banner kind="warn">
          {listing.unreadable_files} instance file(s) could not be read and were skipped.
        </Banner>
      )}
      {error && <Banner kind="error">{error}</Banner>}
      {loading && !listing && <Spinner label="Loading instances…" />}

      {!loading && instances.length === 0 && !error && (
        <EmptyState
          title="No instances yet"
          hint="Create your first Minecraft instance."
          action={
            <Button variant="primary" onClick={openCreate}>
              Create Instance
            </Button>
          }
        />
      )}

      {instances.length > 0 && (
        <ul className="instance-list">
          {instances.map((inst) => {
            const isSelected = config.selected_instance === inst.id;
            return (
              <li key={inst.id} className={`instance-row${isSelected ? " selected" : ""}`}>
                <label className="instance-pick">
                  <input
                    type="radio"
                    name="selected-instance"
                    checked={isSelected}
                    onChange={() => onSelectedChange(isSelected ? null : inst.id)}
                    aria-label={`Select ${inst.name}`}
                  />
                  <span className="instance-name">{inst.name}</span>
                </label>
                <span className="muted">{inst.minecraft_version}</span>
                <LoaderBadge kind={inst.loader.kind} version={inst.loader.version} />
                <span className="muted">created {formatDate(inst.created_at_unix)}</span>
                <span className="instance-actions">
                  <Button variant="ghost" onClick={() => void prepare(inst)} disabled={busy}>
                    {busy ? "Preparing…" : "Download"}
                  </Button>
                  <Button variant="ghost" onClick={() => openEdit(inst)}>
                    Edit
                  </Button>
                  <Button variant="danger" onClick={() => requestDelete(inst)}>
                    Delete
                  </Button>
                </span>
              </li>
            );
          })}
        </ul>
      )}

      {(dialog.kind === "create" || dialog.kind === "edit") && (
        <Dialog
          title={dialog.kind === "create" ? "Create Instance" : `Edit ${dialog.target.name}`}
          onClose={() => setDialog({ kind: "none" })}
        >
          <form
            className="dialog-body"
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
          >
            <Field label="Name" error={errors.name}>
              <input
                value={form.name}
                autoFocus
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="My instance"
                maxLength={80}
              />
            </Field>

            <Field label="Minecraft version" error={errors.minecraftVersion}>
              {dialog.kind === "create" && versionsError ? (
                <>
                  <input
                    value={form.minecraftVersion}
                    onChange={(e) => setForm({ ...form, minecraftVersion: e.target.value })}
                    placeholder={`metadata unavailable — enter manually (${versionsError})`}
                  />
                  <Banner kind="warn">Could not fetch versions: {versionsError}</Banner>
                </>
              ) : dialog.kind === "create" && !versions ? (
                <Spinner label="Fetching official version list…" />
              ) : (
                <>
                  <input
                    value={versionQuery}
                    onChange={(e) => setVersionQuery(e.target.value)}
                    placeholder="Search versions…"
                    aria-label="Filter Minecraft versions"
                  />
                  <select
                    value={form.minecraftVersion}
                    onChange={(e) => setForm({ ...form, minecraftVersion: e.target.value })}
                    size={6}
                    aria-label="Minecraft version list"
                  >
                    {filteredVersions.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.id}
                        {v.kind !== "release" ? ` (${v.kind.replace("_", " ")})` : ""}
                      </option>
                    ))}
                  </select>
                  <label className="field-label">
                    <input
                      type="checkbox"
                      checked={showSnapshots}
                      onChange={(e) => setShowSnapshots(e.target.checked)}
                    />{" "}
                    Show snapshots & legacy versions
                  </label>
                </>
              )}
            </Field>

            <Field label="Mod loader">
              <select
                value={form.loaderKind}
                onChange={(e) =>
                  setForm({ ...form, loaderKind: e.target.value as LoaderKind, loaderVersion: "" })
                }
              >
                {[...SUPPORTED_LOADERS, ...PLANNED_LOADERS].map((k) => (
                  <option key={k} value={k} disabled={PLANNED_LOADERS.includes(k)}>
                    {labelForLoader(k)}
                    {PLANNED_LOADERS.includes(k) ? " — not yet supported" : ""}
                  </option>
                ))}
              </select>
            </Field>

            {loaderNeedsVersion(form.loaderKind) && (
              <Field
                label={`${labelForLoader(form.loaderKind)} version`}
                error={errors.loaderVersion}
              >
                {loaderVersionsState === "loading" ? (
                  <Spinner label={`Fetching ${labelForLoader(form.loaderKind)} versions…`} />
                ) : loaderVersions.length === 0 ? (
                  <>
                    <input
                      value={form.loaderVersion}
                      onChange={(e) => setForm({ ...form, loaderVersion: e.target.value })}
                      placeholder={
                        form.minecraftVersion
                          ? `no compatible versions found for ${form.minecraftVersion}`
                          : "pick a Minecraft version first"
                      }
                    />
                    {form.minecraftVersion && loaderVersionsState === "idle" && (
                      <Banner kind="warn">
                        No stable {labelForLoader(form.loaderKind)} builds are listed for{" "}
                        {form.minecraftVersion}. It may be unsupported by this loader.
                      </Banner>
                    )}
                  </>
                ) : (
                  <select
                    value={form.loaderVersion}
                    onChange={(e) => setForm({ ...form, loaderVersion: e.target.value })}
                  >
                    <option value="">Select a version…</option>
                    {loaderVersions.map((v) => (
                      <option key={v.version} value={v.version}>
                        {v.version}
                      </option>
                    ))}
                  </select>
                )}
              </Field>
            )}

            <div className="dialog-actions">
              <Button variant="ghost" onClick={() => setDialog({ kind: "none" })}>
                Cancel
              </Button>
              <Button variant="primary" type="submit" disabled={busy}>
                {busy ? "Saving…" : dialog.kind === "create" ? "Create" : "Save"}
              </Button>
            </div>
          </form>
        </Dialog>
      )}

      {dialog.kind === "delete" && (
        <ConfirmDialog
          title={`Delete ${dialog.target.name}?`}
          body="This permanently removes the instance entry. The game directory is untouched."
          confirmLabel="Delete"
          busy={busy}
          onConfirm={() => void removeInstance(dialog.target)}
          onCancel={() => setDialog({ kind: "none" })}
        />
      )}
    </section>
  );
}
