# Adding a Loader (contributor guide)

1. **Add the enum variant** — `ikk-core::instance::LoaderKind` (serde lowercase).
2. **Add the `LoaderId` mapping** — `ikk_minecraft::loaders::LoaderId` + the
   match arms in the shell's `loader_id_of`/`parse_loader_id`.
3. **Implement `LoaderProvider`** — a new module in `crates/ikk-minecraft/src/loaders.rs`:
   - `list_versions(agent, mc_version)` — query the loader's official meta API
   - `resolve(agent, mc_version, loader_version, vanilla_json)` — return the
     *effective* metadata (overlay vanilla via `inheritsFrom` when the loader
     publishes merged profiles; otherwise merge libraries/main-class yourself)
4. **Register in `provider_for`** — that single function is the routing table.
5. **Tests** — offline fixtures for parsing; the pipeline (resolve → plan →
   download → launch plan) is already covered by the shared engine tests.
6. **Docs** — update the status table in `docs/architecture/loaders.md`. Mark
   EXPERIMENTAL until a real launch has happened, SUPPORTED only after that.

Rules:
- No loader branches outside `loaders.rs` and the two routing functions.
- No scraping; official APIs with an identifying User-Agent only.
- Never claim support in docs before a real launch proves it.
