# AGENTS.md

herdr-nvim-j-edition is a fork of
[ChmaraX/herdr-nvim](https://github.com/ChmaraX/herdr-nvim) (remote
`upstream`). What it changes is in README.md › What's different from upstream.

## Versioning

Every release is tagged `v<upstream>-mirror-<n>`:

- `<upstream>` is the upstream release this fork is based on (e.g. `1.3.1`).
  It changes only when upstream is merged in.
- `<n>` is this fork's release number on that base: 1, 2, 3, …. It starts at
  1 again whenever `<upstream>` changes.

Examples: `v1.3.1-mirror-1`, `v1.3.1-mirror-2`; after merging upstream
1.4.0, `v1.4.0-mirror-1`.

- The version in `Cargo.toml` and `herdr-plugin.toml` is the tag without the
  `v` (`1.3.1-mirror-2`), and `Cargo.lock` follows (`cargo build`).
- `v<upstream>` tags without `-mirror-` mark the upstream commit a base came
  from (`v1.3.1` = upstream's 1.3.1). They are never released.
- Don't use release-please or any version a tool computes: the number is set
  by hand, by these rules.

## Releasing

1. Bump the version in `Cargo.toml` and `herdr-plugin.toml`, then run
   `cargo build` so that `Cargo.lock` follows.
2. Add a `## [<version>](https://github.com/JeHwanYoo/herdr-nvim-j-edition/compare/<previous tag>...v<version>) (<date>)`
   section at the top of `CHANGELOG.md`. Its body becomes the release notes.
3. Run `just ci`, commit (`chore(release): <version>`), and push `main`.
4. Tag that commit and push the tag: `git tag v<version> && git push origin v<version>`.

The pushed tag runs `.github/workflows/release.yml`, which does two things:

- creates the GitHub release from the CHANGELOG section;
- builds every platform's binary and the app bundles
  (`herdr-nvim-app-<target>.tar.gz` and `.sha256`) and attaches them to the
  release.

The root `install.sh` installs the latest release.

## Following upstream

```sh
git fetch upstream --tags
git merge upstream/main     # or the upstream release tag
```

Keep this fork's changes. Upstream's app features that need a layout edit
(the native tree, its sync controller, the pane header and preview) stay out.
Then tag the merged upstream commit `v<upstream>` and release as
`v<upstream>-mirror-1`.

## Commits

Conventional Commits (`.githooks/commit-msg`, `just setup` installs it).
Before pushing, run `just ci` (cargo fmt check, cargo test, headless Lua
tests) and `herdr-app check herdr-app/mods/<mod>` for each changed mod.
