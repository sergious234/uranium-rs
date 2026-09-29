# AGENTS.md — uranium-rs

Rust library (`edition = "2024"`, `#![forbid(unsafe_code)]`). No binary. Entry: `src/lib.rs`.

## Dependency source

- `mine_data_structs = { version = "1.0.12", features = ["serde"] }` (`Cargo.toml:19`) comes from crates.io — no sibling checkout needed, safe as a git submodule. Manifest types (`RinthModpack`, `PackMeta`, `Side`) live there — do not extend that crate; vendor response structs stay local to loader files.

## Commands

- Fast verify: `cargo check` then `cargo test --lib` (31 unit tests).
- Offline integration: `cargo test --test maker` (uses `tempfile`, no network).
- Single test: `cargo test --lib <name>` or `cargo test --test maker make_with_extension`.
- Gated tests: `cargo test --features integration-tests --test <name>` (`#![cfg(feature = "integration-tests")]`). Cheap (metadata only): `minecraft_progress`, `rinth_vanilla_progress`, `rinth_loader`. Expensive, avoid unless asked: `installation_verification` (downloads real Minecraft `1.21.7`), `runtime`, `rinth_installer`.
- Docs: `cargo doc --no-deps` (1 known warning: bare URL in `engine/downloader.rs`).
- CI (`.github/workflows/rust.yml`): `cargo build`, `cargo test`, `cargo fmt` — all on **nightly**.

## Formatter (non-obvious)

- `rustfmt.toml` sets `unstable_features = true` → stable `cargo fmt` fails. Use `cargo +nightly fmt`. Keep `max_width = 100`, `StdExternalCrate` import grouping, `// region: --- X` / `// endregion` markers.

## Architecture

- `src/engine/` — `FileDownloader` trait + `Downloader` (`from_config(DownloaderConfig)` preferred). Writes are truncate-in-place (mode bits survive); empty files never hash-match, so they stay queued.
- `src/minecraft/installer.rs` — `MinecraftDownloader<T: FileDownloader + Send>` facade; phases in `assets.rs`, `client.rs`, `libraries.rs`, `runtime/`, `verify/`, dispatched via `steps.rs:InstallStep/InstallCtx`. States: `GettingSources → DownloadingVersion → DownloadingAssets → DownloadingLibraries → DownloadingRuntime → CheckingFiles → Completed`. Only the asset/library arms drain per-batch via `downloader.progress()`; `DownloadingRuntime` (`RuntimeDownloader::start_with`) is still a one-shot drain — known gap, do not "fix" unasked.
- `src/modpacks/rinth/installer.rs` — `RinthInstaller<T: FileDownloader>` facade with `progress()`/`start()`; `Side` fixed at construction; terminal state always `Completed`, launch target via `installed_profile_id()` getter, phase strings via `RinthInstallState::as_str()`. `VanillaStep` is stateful (persisted `vanilla` field, ctx built inline — `ctx()` takes `&mut self`); `requests_left()` delegates to the vanilla queue during `InstallingMinecraft`.
- `src/modpacks/{rinth,curse}/` — separate products, intentionally **not** deduplicated. New source = new subdir + register in `src/modpacks/mod.rs` (OCP). Canonical `ensure_pack_dirs` is `modpacks::common` (re-exported via `modpacks/mod.rs`); `crate::common` holds only `constants`/`fs`/`hash`.
- `src/modpacks/rinth/loaders/` — Fabric/Quilt install from vendor `/profile/json`; Forge/NeoForge are stubs returning "not supported yet" errors.
- `src/config.rs` — `DownloaderConfig::new/with_max_concurrent` is current; global `NTHREADS`/`num_threads()`/`set_threads()` are deprecated shims.

## Conventions from recent refactors (honor these)

- DIP: `with_downloader(.., dl: T)` + pure `from_pack` ctors; inject temp dirs via `unzip_temp_pack_at(zip, tmp)`, never hardcode `TEMP_DIR` in new logic. No `Send+Sync` bounds in rinth steps unless the compiler demands them (the minecraft installer still carries legacy `Send`/`Sync` bounds — leave them).
- Progress rule: steps only `add_object`; draining happens exclusively in per-batch `progress()` arms. Never call `start()` inside a step — it silently downloads the whole shared queue (this regressed before; `minecraft_progress` guards it).
- Chunk math: use `.div_ceil(num_threads())` (ceil, not floor).
- Docs are functional (what it does for a launcher dev), not impl-how. `pub` API gets `# Example` (`no_run`) except trivial getters (`chunks`, `requests_left`, `finished`, `get_modpack_name` — no example). Internals: `pub(crate)` + `#[doc(hidden)]` one-liner; private fns undocumented. Keep `# Errors` generic in `rinth/`.

## Gotchas

- `TEMP_DIR` is a global `LazyLock` (OS state dir or `tmp/uranium`); each installer gets a unique `mrpack-<pid>-<n>` subdir and the private `cleanup()` must run on completion/error or it leaks. Tests should use `*_at` variants with a `tempdir`.
- Client-jar permissions: `download_client` pre-creates the empty jar and sets the exec bit at queue time (bit survives the later truncate-in-place write); `check_client` re-asserts the bit when the jar already exists.
- `CurseDownloader::new` is `async` and does network I/O; `RinthInstaller::new` is sync. Curse reads `CURSE_API_KEY` env (empty string if unset → API calls fail). Do not touch `curse/` unless asked.
- `.gitignore` lists `data/*` and `tests/*` but fixtures (`tests/data/minecraft_test1/`, `test1.mrpack`) exist in-tree — do not "fix" this unasked.
