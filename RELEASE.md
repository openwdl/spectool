# Release

  * [ ] Update version in `Cargo.toml`.
  * [ ] Run tests: `cargo test`.
  * [ ] Run linting: `cargo clippy -- -D warnings`.
  * [ ] Run fmt: `cargo +nightly fmt --check`.
  * [ ] Run doc: `cargo doc`.
  * [ ] Update `CHANGELOG.md` with the new version and publication date.
  * [ ] Update the `[unreleased]` link at the bottom of `CHANGELOG.md` to
    compare against the new version tag.
  * [ ] Add a new version link at the bottom of `CHANGELOG.md`.
  * [ ] Stage changes: `git add Cargo.toml Cargo.lock CHANGELOG.md`.
  * [ ] Create git commit:
    ```
    chore: releases `v0.X.Y` of `spectool`
    ```
  * [ ] Create git tag:
    ```
    git tag v0.X.Y
    ```
  * [ ] Push the release: `git push && git push --tags`.
  * [ ] Make sure the CI is green for the tag push.
  * [ ] Verify the `release.yml` workflow creates the GitHub Release with
    binaries for all three platforms (Linux, macOS, Windows).
  * [ ] Go to the Releases page on GitHub and add release notes from the
    `CHANGELOG.md` entry.
