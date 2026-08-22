// Instances — the first real screen (spec §15). All mutations go through the
// typed API (application layer); this view never touches persistence itself.
import { useMemo, useState } from "react";
import {
  createInstance,
  deleteInstance,
  toErrorMessage,
  updateInstance,
  type AppConfig,
  type Instance,
  type InstanceListing,
  type LoaderKind,
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

const LOADER_KINDS: readonly LoaderKind[] = ["vanilla", "fabric", "forge", "neoforge", "quilt"];

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

  const instances = useMemo(() => listing?.instances ?? [], [listing]);

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
          <p className="muted">Isolated Minecraft environments. Launching arrives in a later milestone.</p>
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
        <Dialog title={dialog.kind === "create" ? "Create Instance" : `Edit ${dialog.target.name}`} onClose={() => setDialog({ kind: "none" })}>
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
              <input
                value={form.minecraftVersion}
                onChange={(e) => setForm({ ...form, minecraftVersion: e.target.value })}
                placeholder="e.g. 1.21.x (metadata discovery comes later)"
              />
            </Field>
            <Field label="Mod loader">
              <select
                value={form.loaderKind}
                onChange={(e) => setForm({ ...form, loaderKind: e.target.value as LoaderKind })}
              >
                {LOADER_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {labelForLoader(k)}
                  </option>
                ))}
              </select>
            </Field>
            {loaderNeedsVersion(form.loaderKind) && (
              <Field label={`${labelForLoader(form.loaderKind)} version`} error={errors.loaderVersion}>
                <input
                  value={form.loaderVersion}
                  onChange={(e) => setForm({ ...form, loaderVersion: e.target.value })}
                  placeholder="Loader version"
                />
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
