# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.7] - 2026-01-08

### Changed

- Changes badge color threshold from `95%` back to `100%` conformance for `brightgreen`.

## [0.1.6] - 2026-01-04

### Changed

- Changes badge color threshold from `100%` to `95%` conformance for `brightgreen`.

## [0.1.5] - 2026-01-03

### Changed

- Splits timing statistics into four categories based on expected vs. actual
  test outcomes (expected pass/test pass, expected pass/test fail, expected
  fail/test pass, expected fail/test fail).

## [0.1.4] - 2026-01-03

### Added

- Added multicore support for `test`. By default, all cores are now used
  (configurable with the `--n-cpu` flag).

### Changed

- Changes test performance metric from average time to median time.

## [0.1.3] - 2026-01-02

### Changed

- Adds a `--version` argument.

## [0.1.2] - 2026-01-01

### Changed

- Tests failing no longer returns a non-zero exit code (the command still
  worked) successfully.

## [0.1.1] - 2026-01-01

### Changed

- Adds the remote repository option.

## [0.1.0] - 2026-01-01

### Added

- Initial version released.

[unreleased]: https://github.com/openwdl/spectool/compare/v0.1.7...HEAD
[0.1.7]: https://github.com/openwdl/spectool/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/openwdl/spectool/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/openwdl/spectool/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/openwdl/spectool/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/openwdl/spectool/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/openwdl/spectool/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/openwdl/spectool/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/openwdl/spectool/releases/tag/v0.1.0
