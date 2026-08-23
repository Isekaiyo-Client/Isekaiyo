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

/** Ask the core to launch an instance with an offline-profile identity.
 *  Resolves with the game PID once the process starts. */
export function launchInstance(id: string, username: string): Promise<number> {
  return invoke<number>("launch_instance", { id, username });
}

// -- version & loader metadata (Phase 3/5) ------------------------------------

export interface VersionEntryDto {
  id: string;
  kind: "release" | "snapshot" | "old_beta" | "old_alpha";
}

export interface VersionListDto {
  source: "cache" | "network" | "stale-cache";
  entries: VersionEntryDto[];
}

export function listVersions(forceRefresh = false): Promise<VersionListDto> {
  return invoke<VersionListDto>("list_versions", { forceRefresh });
}

export interface LoaderVersionDto {
  version: string;
  stable: boolean;
}

export function listLoaderVersions(kind: string, mcVersion: string): Promise<LoaderVersionDto[]> {
  return invoke<LoaderVersionDto[]>("list_loader_versions", { kind, mcVersion });
}

export interface InstallReportDto {
  downloaded: number;
  skipped: number;
  total_files: number;
  failed: string[];
}

export function installInstance(id: string): Promise<InstallReportDto> {
  return invoke<InstallReportDto>("install_instance", { id });
}

export interface GameExitDto {
  exit_code: number | null;
  user_stopped: boolean;
  category: "completed" | "crashed" | "user-stopped";
}

export interface LaunchStatusDto {
  phase: string;
  pid: number | null;
  exit: GameExitDto | null;
  log_path: string | null;
}

export function launchStatus(): Promise<LaunchStatusDto> {
  return invoke<LaunchStatusDto>("launch_status");
}

export function stopLaunch(): Promise<boolean> {
  return invoke<boolean>("stop_launch");
}

export function readLaunchLog(maxBytes = 65536): Promise<string> {
  return invoke<string>("read_launch_log", { maxBytes });
}

// -- mod management (Phase 6) -------------------------------------------------
// Remote marketplace data (ModProjectDto) and local installation state
// (InstalledModDto) are separate shapes — mirroring the domain rule that
// they are never one object.

export interface ModProjectDto {
  source: string;
  project_id: string;
  title: string;
  description: string;
  authors: string[];
  icon_url: string | null;
  downloads: number;
  categories: string[];
  game_versions: string[];
}

export interface ModVersionDto {
  version_id: string;
  version_number: string;
  release_type: "release" | "beta" | "alpha" | string;
  filename: string;
  size_bytes: number;
  hash_verified_source: boolean;
}

export interface ModInstallPlanDto {
  to_install: ModProjectDto[];
  already_installed: string[];
  unsatisfiable: string[];
  conflicts: string[];
}

export interface ModInstallReportDto {
  downloaded: string[];
  skipped: string[];
  unverified: string[];
  failed: string[];
}

export type ModState = "managed" | "external" | "missing";

export interface InstalledModDto {
  source: string;
  project_id: string | null;
  title: string;
  filename: string;
  version_number: string | null;
  enabled: boolean;
  state: ModState;
  warning: string | null;
}

export interface ModInventoryDto {
  mods: InstalledModDto[];
}

export interface ModProfileDto {
  id: string;
  name: string;
  enabled_count: number;
  active: boolean;
}

export interface ModUpdateDto {
  project_id: string;
  installed_version: string;
  available_version: string | null;
  state: "current" | "update-available" | "incompatible" | "unknown";
}

/** Search a mod source scoped to the instance's Minecraft version + loader. */
export function modsSearch(
  instanceId: string,
  query: string,
  sort?: string,
  page = 1,
): Promise<ModProjectDto[]> {
  return invoke<ModProjectDto[]>("mods_search", { instanceId, query, sort, page });
}

export function modsCompatibleVersions(
  instanceId: string,
  source: string,
  projectId: string,
): Promise<ModVersionDto[]> {
  return invoke<ModVersionDto[]>("mods_compatible_versions", { instanceId, source, projectId });
}

/** Resolve what an install would do WITHOUT downloading anything. */
export function modsInstallPlan(
  instanceId: string,
  source: string,
  projectId: string,
): Promise<ModInstallPlanDto> {
  return invoke<ModInstallPlanDto>("mods_install_plan", { instanceId, source, projectId });
}

export function modsInstall(
  instanceId: string,
  source: string,
  projectId: string,
): Promise<ModInstallReportDto> {
  return invoke<ModInstallReportDto>("mods_install", { instanceId, source, projectId });
}

export function modsInventory(instanceId: string): Promise<ModInventoryDto> {
  return invoke<ModInventoryDto>("mods_inventory", { instanceId });
}

export function modsSetEnabled(
  instanceId: string,
  projectId: string,
  enabled: boolean,
): Promise<void> {
  return invoke<void>("mods_set_enabled", { instanceId, projectId, enabled });
}

export function modsRemove(
  instanceId: string,
  projectId: string,
  force = false,
): Promise<void> {
  return invoke<void>("mods_remove", { instanceId, projectId, force });
}

export function modsUpdates(instanceId: string): Promise<ModUpdateDto[]> {
  return invoke<ModUpdateDto[]>("mods_updates", { instanceId });
}

export function modsListProfiles(instanceId: string): Promise<ModProfileDto[]> {
  return invoke<ModProfileDto[]>("mods_list_profiles", { instanceId });
}

export function modsCreateProfile(instanceId: string, name: string): Promise<ModProfileDto[]> {
  return invoke<ModProfileDto[]>("mods_create_profile", { instanceId, name });
}

export function modsSwitchProfile(instanceId: string, profileId: string | null): Promise<void> {
  return invoke<void>("mods_switch_profile", { instanceId, profileId });
}
