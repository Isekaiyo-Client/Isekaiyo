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
  updated_at_unix: number;
  settings: LaunchSettings;
}

export interface InstanceListing {
  instances: Instance[];
  unreadable_files: number;
}

export interface LoaderInput {
  kind: LoaderKind;
  version: string | null;
}

/** Per-instance launch preferences (Phase 8 §3) — typed, optional fields so
 *  older persisted instances deserialize with defaults. */
export interface LaunchSettings {
  memory_mb: number | null;
  min_memory_mb: number | null;
  window_width: number | null;
  window_height: number | null;
  fullscreen: boolean;
  jvm_args: string[];
  game_args: string[];
  env: Record<string, string>;
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

// -- instance engine (Phase 8) -------------------------------------------------

/** Duplicate an instance's metadata into a fresh id. */
export function duplicateInstance(id: string, newName?: string): Promise<Instance> {
  return invoke<Instance>("duplicate_instance", { id, newName: newName ?? null });
}

export function renameInstance(id: string, name: string): Promise<Instance> {
  return invoke<Instance>("rename_instance", { id, name });
}

/** Safe delete — the file moves to trash; game data is untouched. */
export function trashDeleteInstance(id: string): Promise<string> {
  return invoke<string>("trash_delete_instance", { id });
}

export interface FindingDto {
  severity: "warning" | "error";
  code: string;
  path: string | null;
  message: string;
}

export interface RepairActionDto {
  kind: "redownload" | "create-directory";
  url: string | null;
  dest: string;
  sha1: string | null;
}

export interface ValidationReportDto {
  ok: boolean;
  findings: FindingDto[];
  repairs: RepairActionDto[];
}

export function validateInstance(id: string): Promise<ValidationReportDto> {
  return invoke<ValidationReportDto>("validate_instance", { id });
}

/** Run the repair actions validation proposed; returns how many were applied. */
export function repairInstance(id: string): Promise<number> {
  return invoke<number>("repair_instance", { id });
}

/** Dry-run launch preview — argv is ALWAYS redacted backend-side. */
export interface DryRunLaunchDto {
  java_executable: string;
  main_class: string;
  jvm_args: string[];
  game_args: string[];
  argv_redacted: string[];
  game_dir: string;
  assets_dir: string;
}

export function dryRunLaunch(id: string, username: string): Promise<DryRunLaunchDto> {
  return invoke<DryRunLaunchDto>("dry_run_launch", { id, username });
}

export interface DirSizeDto {
  label: string;
  path: string;
  bytes: number;
}

export interface StorageReportDto {
  instances: DirSizeDto[];
  cache: DirSizeDto[];
  total_bytes: number;
}

export function storageUsage(): Promise<StorageReportDto> {
  return invoke<StorageReportDto>("storage_usage");
}

export type TaskState = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface TaskSnapshotDto {
  id: string;
  label: string;
  state: TaskState;
  current: number;
  total: number;
  percent: number;
  message: string;
  error_code: string | null;
  error_message: string | null;
}

export function taskStatus(): Promise<TaskSnapshotDto[]> {
  return invoke<TaskSnapshotDto[]>("task_status");
}

// -- accounts (Phase 9) ---------------------------------------------------------
// These DTOs carry PUBLIC metadata only — the backend model has no credential
// fields, and no command exists that returns any token.

export type AccountKind = "microsoft" | "offline";
export type AccountStatus =
  | "signed-out"
  | "authenticated"
  | "refreshing"
  | "expired"
  | "reauth-required"
  | "error";

export interface AccountDto {
  id: string;
  kind: AccountKind;
  display_name: string;
  username: string;
  uuid: string;
  avatar_url: string | null;
  status: AccountStatus;
  active: boolean;
}

export interface DeviceCodeDto {
  /** Not a secret (OAuth device grant): used to poll account_microsoft_poll. */
  device_code: string;
  user_code: string;
  verification_uri: string;
  interval_secs: number;
}

export function accountList(): Promise<AccountDto[]> {
  return invoke<AccountDto[]>("account_list");
}

export function accountGetActive(): Promise<AccountDto | null> {
  return invoke<AccountDto | null>("account_get_active");
}

/** Create an explicit Offline/Local profile (stable UUID from the username). */
export function accountAddOffline(displayName: string, username: string): Promise<AccountDto> {
  return invoke<AccountDto>("account_add_offline", { displayName, username });
}

/** Step 1 of Microsoft sign-in: get the REAL device code to show the user. */
export function accountMicrosoftStart(): Promise<DeviceCodeDto> {
  return invoke<DeviceCodeDto>("account_microsoft_start");
}

/** One poll of the device flow; retry at `interval_secs` while state is pending. */
export function accountMicrosoftPoll(
  deviceCode: string,
): Promise<[state: string, account: AccountDto | null]> {
  return invoke<[string, AccountDto | null]>("account_microsoft_poll", { deviceCode });
}

export function accountSelect(id: string): Promise<void> {
  return invoke<void>("account_select", { id });
}

/** Remove account: credentials deleted first, then metadata. */
export function accountRemove(id: string): Promise<void> {
  return invoke<void>("account_remove", { id });
}

/** Sign out: credentials removed, harmless metadata kept. */
export function accountLogout(id: string): Promise<void> {
  return invoke<void>("account_logout", { id });
}

/** One bounded silent-refresh attempt; errors mean reauthentication. */
export function accountRefresh(id: string): Promise<AccountDto> {
  return invoke<AccountDto>("account_refresh", { id });
}

/** Launch now resolves the ACTIVE account backend-side — no username param. */
export function launchInstance(id: string): Promise<number> {
  return invoke<number>("launch_instance", { id });
}
