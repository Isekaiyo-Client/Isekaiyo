// Pure validation for the instance form — no React, no Tauri, fully testable.
// Mirrors the server-side invariants in ikk-core::instance (MAX_NAME_LEN = 64).

export const MAX_INSTANCE_NAME_LEN = 64;

export interface InstanceForm {
  name: string;
  minecraftVersion: string;
  loaderKind: "vanilla" | "fabric" | "forge" | "neoforge" | "quilt";
  loaderVersion: string;
}

export type FormErrors = Partial<Record<keyof InstanceForm, string>>;

/** Returns field errors; an empty object means the form is valid. */
export function validateInstanceForm(form: InstanceForm): FormErrors {
  const errors: FormErrors = {};
  const name = form.name.trim();
  if (!name) errors.name = "Name is required.";
  else if (name.length > MAX_INSTANCE_NAME_LEN)
    errors.name = `Name must be at most ${MAX_INSTANCE_NAME_LEN} characters.`;

  if (!form.minecraftVersion.trim()) errors.minecraftVersion = "Minecraft version is required.";

  if (!loaderNeedsVersion(form.loaderKind)) return errors;
  if (!form.loaderVersion.trim())
    errors.loaderVersion = `${labelForLoader(form.loaderKind)} requires a version.`;
  return errors;
}

/** Non-vanilla loaders must declare a version — same rule as ikk-core. */
export function loaderNeedsVersion(kind: InstanceForm["loaderKind"]): boolean {
  return kind !== "vanilla";
}

const LOADER_LABELS: Record<InstanceForm["loaderKind"], string> = {
  vanilla: "Vanilla",
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
  quilt: "Quilt",
};

export function labelForLoader(kind: InstanceForm["loaderKind"]): string {
  return LOADER_LABELS[kind];
}
