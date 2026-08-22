// Typed mirror of crates/ikk-api-types — the ONLY way the UI talks to core.
// No fetch, no fs, no process spawning outside these functions (dependency
// rule 3, docs/architecture/dependency-rules.md).
// TODO(M1.2): generate this file from Rust (`specta`/`ts-rs`) so the mirror can
// never drift — tracked in docs/roadmap.md.
import { invoke } from "@tauri-apps/api/core";

export interface SystemInfo {
  app_version: string;
  target: string;
  profile: string;
}

export type Theme = "amoled" | "modern" | "sakura";
export type StartPage = "home" | "instances";
/** serde rename_all=lowercase of LoadSourceDto — matches the Rust contract. */
export type LoadSourceDto = "defaults" | "file" | "recoveredcorrupt";

export interface AppConfig {
  schema_version: number;
  theme: Theme;
  start_page: StartPage;
  selected_instance: string | null;
  confirm_before_delete: boolean;
  animations_enabled: boolean;
}

export interface ConfigLoadInfo {
  source: LoadSourceDto;
  corrupt_backup_path: string | null;
}

export type LoaderKind = "vanilla" | "fabric" | "forge" | "neoforge" | "quilt";

export interface LoaderSpec {
  kind: LoaderKind;
  version: string | null;
}

export interface Instance {
  id: string;
  name: string;
  minecraft_version: string;
  loader: LoaderSpec;
  created_at_unix: number;
  last_played_unix: number | null;
}

export interface InstanceListing {
  instances: Instance[];
  unreadable_files: number;
}

export interface LoaderInput {
  kind: LoaderKind;
  version: string | null;
}

/** Serializable projection of ikk-core::Error (stable `code` categories).
 *  `runtime.unavailable` = domain exists, runtime implementation does not yet. */
export interface CommandError {
  code: string;
  message: string;
}

/** Normalize anything a rejected invoke gives us into displayable text. */
export function toErrorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    const ce = e as CommandError;
    return "code" in e ? `[${ce.code}] ${ce.message}` : String(ce.message);
  }
  return e instanceof Error ? e.message : String(e);
}

// -- commands ----------------------------------------------------------------

export function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>("get_system_info");
}

export function getStartupInfo(): Promise<ConfigLoadInfo> {
  return invoke<ConfigLoadInfo>("get_startup_info");
}

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export function setConfig(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>("set_config", { config });
}

export function listInstances(): Promise<InstanceListing> {
  return invoke<InstanceListing>("list_instances");
}

export function createInstance(
  name: string,
  minecraftVersion: string,
  loader: LoaderInput | null,
): Promise<Instance> {
  return invoke<Instance>("create_instance", {
    name,
    minecraftVersion,
    loader,
  });
}

export function updateInstance(instance: Instance): Promise<Instance> {
  return invoke<Instance>("update_instance", { instance });
}

export function deleteInstance(id: string): Promise<boolean> {
  return invoke<boolean>("delete_instance", { id });
}

/** Ask the core to launch an instance. Resolves only if a runtime exists;
 *  today it rejects with `code: "runtime.unavailable"` (honest by design). */
export function launchInstance(id: string): Promise<void> {
  return invoke<void>("launch_instance", { id });
}
