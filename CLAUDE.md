# amiga-tools

Host-side CLI tools for classic Amiga development, in the spirit of amitools'
command-line tooling (xdftool, rdbtool, romtool, ...), but built in Rust on
Simon's published format crates. Other languages/tools may join later.

## Layout

- Cargo workspace; each tool is a binary crate under `tools/<name>/`.
- Workspace-level deps live in `[workspace.dependencies]` in the root
  Cargo.toml; tool crates inherit package metadata via `*.workspace = true`.

## Foundation crates (published on crates.io, sources checked out locally)

- `amiga-ffs` (~/src/amiga-ffs-rs) — OFS/FFS volumes: read, validate, format,
  mutate, resize, defragment. Backs `amidisk`, the xdftool equivalent.
- `amiga-rdb` (~/src/amiga-rdb-rs) — Rigid Disk Block partition tables.
  Backs `amirdb`, the rdbtool equivalent.
- `amiga-rom` (~/src/amiga-rom-rs-) — Kickstart ROM images: inspect,
  normalize, split/merge. Backs `amirom`, the romtool equivalent.

All three are deliberately dependency-free, no_std-capable format libraries;
this repo is where the real CLI/IO layer lives. When a tool needs a format
capability the crate lacks, add it to the crate (in its own repo), not here.

## Conventions

- clap v4 derive, subcommand style; anyhow for error context. No
  unwrap/expect outside genuinely impossible cases.
- Comment style matches the foundation crates: terse and purposeful, only
  where the code can't say it.
- amitools (Python) is the behavioural oracle: when in doubt about output or
  semantics, compare against the real xdftool/rdbtool/romtool.
- Never commit ROM images or other copyrighted Amiga material.

## Verification

`cargo build`, `cargo clippy -- -D warnings`, `cargo test` from the repo
root. Smoke-test tools against scratch images (create via the crates' own
format/create APIs or amitools) in a temp dir, never in the repo.
