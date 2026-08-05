# Changelog

## [0.1.5](https://github.com/terror/ocs/releases/tag/0.1.5) - 2026-08-05

### Added

- Add `zsh` shell integration ([#29](https://github.com/terror/ocs/pull/29) by [terror](https://github.com/terror))
- Add `bash` shell integration ([#30](https://github.com/terror/ocs/pull/30) by [terror](https://github.com/terror))
- Open selected sessions immediately ([#33](https://github.com/terror/ocs/pull/33) by [terror](https://github.com/terror))

### Fixed

- Change session search binding ([#31](https://github.com/terror/ocs/pull/31) by [terror](https://github.com/terror))

### Misc

- Update demo image in readme ([#28](https://github.com/terror/ocs/pull/28) by [terror](https://github.com/terror))
- Assert full sessions in storage tests ([#32](https://github.com/terror/ocs/pull/32) by [terror](https://github.com/terror))

## [0.1.4](https://github.com/terror/ocs/releases/tag/0.1.4) - 2026-08-03

### Added

- Validate opencode database schema ([#19](https://github.com/terror/ocs/pull/19) by [terror](https://github.com/terror))
- Allow configuring opencode session arguments ([#26](https://github.com/terror/ocs/pull/26) by [terror](https://github.com/terror))

### Fixed

- Make `--database` and `--data-dir` conflicting ([#20](https://github.com/terror/ocs/pull/20) by [terror](https://github.com/terror))
- Forward database when opening sessions ([#21](https://github.com/terror/ocs/pull/21) by [terror](https://github.com/terror))
- Normalize database paths ([#24](https://github.com/terror/ocs/pull/24) by [terror](https://github.com/terror))
- Separate project with dots in session rows ([#25](https://github.com/terror/ocs/pull/25) by [terror](https://github.com/terror))

### Misc

- Derive `Default` for `Session` ([#22](https://github.com/terror/ocs/pull/22) by [terror](https://github.com/terror))
- Sort tests alphabetically ([#23](https://github.com/terror/ocs/pull/23) by [terror](https://github.com/terror))

## [0.1.3](https://github.com/terror/ocs/releases/tag/0.1.3) - 2026-07-31

### Added

- Filter out subagent sessions ([#12](https://github.com/terror/ocs/pull/12) by [terror](https://github.com/terror))
- Default to sessions in the current directory ([#13](https://github.com/terror/ocs/pull/13) by [terror](https://github.com/terror))
- Show usage metadata in session rows ([#17](https://github.com/terror/ocs/pull/17) by [terror](https://github.com/terror))

### Fixed

- Delete sessions through opencode directly ([#14](https://github.com/terror/ocs/pull/14) by [terror](https://github.com/terror))
- Make transcript ordering deterministic ([#16](https://github.com/terror/ocs/pull/16) by [terror](https://github.com/terror))

### Misc

- Bump skim from 5.5.0 to 5.6.1 ([#11](https://github.com/terror/ocs/pull/11) by [app/dependabot](https://github.com/app/dependabot))

## [0.1.2](https://github.com/terror/ocs/releases/tag/0.1.2) - 2026-07-27

### Added

- Add version flag ([#9](https://github.com/terror/ocs/pull/9) by [terror](https://github.com/terror))

### Misc

- Bump skim from 5.1.3 to 5.5.0 ([#7](https://github.com/terror/ocs/pull/7) by [app/dependabot](https://github.com/app/dependabot))
- Bump anyhow from 1.0.103 to 1.0.104 ([#6](https://github.com/terror/ocs/pull/6) by [app/dependabot](https://github.com/app/dependabot))
- Bump clap from 4.6.2 to 4.6.4 ([#5](https://github.com/terror/ocs/pull/5) by [app/dependabot](https://github.com/app/dependabot))
- Rename session lookup method ([#8](https://github.com/terror/ocs/pull/8) by [terror](https://github.com/terror))

## [0.1.1](https://github.com/terror/ocs/releases/tag/0.1.1) - 2026-07-17

### Added

- Add current directory filter ([#2](https://github.com/terror/ocs/pull/2) by [terror](https://github.com/terror))
- Add support for deleting sessions ([#1](https://github.com/terror/ocs/pull/1) by [terror](https://github.com/terror))

### Misc

- Extract terminal styles ([#3](https://github.com/terror/ocs/pull/3) by [terror](https://github.com/terror))

## [0.1.0](https://github.com/terror/ocs/releases/tag/0.1.0) - 2026-07-17

Initial release
