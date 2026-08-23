# Download Engine (Phase 8)

Implementation: `ikk-minecraft::download`. One engine serves versions,
libraries, assets, loader artifacts and mod files — there is exactly one cache
and one download path (spec §35/§60: no parallel systems).

## Guarantees

- **Skip-if-valid** — existing file whose checksum matches is never re-downloaded
- **Atomic finalization** — always download to `<dest>.part`, hash while
  streaming, then rename; a crash can never leave a half file pretending to be valid
- **Checksum verification** — SHA-1 (Mojang metadata) and SHA-256 (§12); a
  mismatch deletes the temp file and reports `download.checksum_mismatch`
- **Bounded retries with exponential backoff** — 250 ms doubling, capped at 4 s;
  never infinite (§63)
- **Cooperative cancellation** — an `AtomicBool` observed between attempts and
  before each retry; cancellation returns `operation.cancelled` and cleans the
  `.part` file (§64)
- **Progress callbacks** — real cumulative byte counts, wired to the task
  manager so the UI sees "Downloading libraries 23 / 148" (§14, §69–§70)

## Error taxonomy (§62)

I/O failures are classified via `ikk_core::classify_io`: disk full
(`io.disk_full`), permission denied (`io.permission_denied`), plain I/O
(`io.failure`), network (`network.timeout`), HTTP status (`metadata.invalid`),
checksum (`download.checksum_mismatch`), cancelled (`operation.cancelled`).
The UI branches on these codes instead of parsing prose.

## Task manager

`ikk-core::tasks::TaskManager` tracks every long operation with id/state/
progress/cancel; terminal tasks are pruned beyond a small ring. The Tauri
command `task_status` exposes snapshots; installs publish progress through it.
