# Contributing

Thanks for your interest in Loud Alarm!

## Getting started

```sh
pnpm install
pnpm tauri dev
```

## Before opening a PR

Please make sure the same checks CI runs pass locally:

```sh
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Conventions

- TypeScript for all frontend code; formatting and linting via [Biome](https://biomejs.dev).
- Rust formatted with `rustfmt`, linted with `clippy` (warnings are errors in CI).
- Keep platform-specific behaviour behind the `volume` module and `cfg` gates.

## Releases

Releases are cut by pushing a tag:

```sh
git tag v0.1.0-beta.1
git push origin v0.1.0-beta.1
```

The `release` workflow builds installers for macOS, Linux, and Windows and
publishes them to a GitHub Release. Tags containing a hyphen (e.g. `-beta.1`)
are published as prereleases.
