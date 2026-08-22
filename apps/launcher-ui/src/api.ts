// Typed mirror of crates/ikk-api-types (SystemInfo).
// TODO(M1.2): generate this file from Rust with `specta`/`ts-rs` so the mirror
// can never drift — tracked in docs/roadmap.md.
import { invoke } from "@tauri-apps/api/core";

export interface SystemInfo {
  app_version: string;
  target: string;
  profile: string;
}

export function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>("get_system_info");
}
