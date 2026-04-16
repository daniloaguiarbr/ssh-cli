# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-04-16

### Added
- Three new error-output helpers in `output.rs` — `imprimir_erro_runtime`, `imprimir_erro_dominio`, `imprimir_erro_generico` — to centralize all terminal I/O inside the single authorized module.
- `imprimir_erro_generico` now iterates the full `anyhow::Error` chain and prints each cause with a `causado por:` prefix for richer debugging.
- Bilingual README sections "Integration patterns" (6 pipeable examples) and "Quick Reference" (17-line consolidated lookup table) in both English and Portuguese.
- Explicit `See CHANGELOG.md` navigation link at the bottom of each README language section.
- 11 `// TESTABILIDADE:` inline comments in `ssh/cliente.rs` documenting which functions require a real `russh::Session` (only covered by future end-to-end tests with an embedded SSH server).
- 50+ new unit and integration tests covering `truncar_utf8` edge cases (CJK, emoji, invariants), `parse_header_scp` parsing, `mapear_exit_status` branches, `ConfiguracaoConexao` clones, `ClienteSsh` mocks via `mockall` (expectations, sequences, predicates), tunnel cancellation via `FLAG_CANCELAMENTO`, and terminal `NO_COLOR` detection.
- Three new end-to-end tests in `tests/e2e_cli.rs` exercising `main.rs` error branches (runtime errors, domain errors, `ErroSshCli` downcasting).
- Detailed `[0.1.0]` changelog entry expanded from `Initial release.` to 22 Added bullets plus 3 Security bullets — full historical inventory of the original feature set.

### Changed
- `main.rs` now delegates all error-path I/O to `output.rs` helpers; the three `eprintln!` calls that violated the "only `output.rs` may touch the terminal" rule have been removed.
- README CI badge URL corrected to the canonical `daniloaguiarbr/ssh-cli` repository (previously pointed to a non-existent `comandoaguiar` org, breaking the badge).
- `#[serial]` attribute added to three `vps/mod.rs` tests that manipulate the global `SSH_CLI_HOME` environment variable (`caminho_config_padrao_com_ssh_cli_home_retorna_path`, `caminho_config_padrao_com_path_traversal_retorna_erro`, `caminho_config_padrao_sem_env_retorna_path_valido`) to eliminate flakiness under parallel execution.
- `#[serial]` also applied to three mock-based VPS execution tests that depend on the global cancellation flag.
- `#[serial]` applied to three `terminal.rs` tests that toggle `NO_COLOR` / `CLICOLOR_FORCE` environment variables.

### Fixed
- Flaky test suite under `cargo test --all-features` — previously 1 test failed intermittently in parallel runs due to shared env-var state; three consecutive full runs now complete with 365 tests passing and 0 failures.
- `main.rs` stderr output no longer bypasses the project-wide formatting policy; error messages route through the same code path that handles `--output-format json` and `--no-color`.

### Security
- Global code coverage raised from 50% (ssh/cliente.rs) and 83.35% (project) to 86.41% regions / 87.93% lines / 89.01% functions — reducing the surface of uncovered execution paths.
- 11 SSH functions that intrinsically require a real `russh::Session` now have documented testability gaps (`// TESTABILIDADE:`) so future auditors understand the coverage limit is a known architectural constraint, not an oversight.

## [0.2.1] - 2026-04-16

### Fixed
- Pin `elliptic-curve = "=0.14.0-rc.30"` to fix `cargo install ssh-cli` failure caused by incompatible `elliptic-curve 0.14.0-rc.31+` being resolved for `p256/p384/p521 0.14.0-rc.8`

## [0.2.0] - 2026-04-15

### Added
- Fix sudo-exec stdin password piping with `printf '%s\n'`
- Runtime overrides: --password, --sudo-password, --timeout flags on exec/sudo-exec/scp/tunnel
- LLM-friendly camelCase aliases (--sudoPassword, --suPassword)

## [0.1.0] - 2026-04-14

### Added
- Initial public release of `ssh-cli` — single static binary, zero runtime dependencies.
- Subcommands: `connect`, `exec`, `sudo-exec`, `scp upload`, `scp download`, `tunnel`, `vps {add,list,show,edit,remove,path}`, `health-check`, `completions`.
- Full Rust SSH stack via `russh 0.60` + `aws-lc-rs` crypto backend — zero C bindings.
- Persistent VPS registry at `$XDG_CONFIG_HOME/ssh-cli/config.toml` with automatic `chmod 0o600` on Unix.
- `VpsRegistro` model with password fields wrapped in `secrecy::SecretString` (Zeroize on Drop).
- Unicode-safe password masking (12 first + 4 last chars; `***` for length ≤ 16) using `chars()` iteration.
- Schema versioning (`schema_version: u32`) and RFC 3339 timestamps (`added_at`).
- Path-traversal protection for `SSH_CLI_HOME` environment override.
- Bilingual i18n scaffolding (`en-US`, `pt-BR`) via `rust-i18n` with `sys-locale` auto-detection and `--lang` override.
- `FormatoSaida` enum — text (default) and JSON output via `--output-format`.
- Platform init layer: Windows calls `SetConsoleOutputCP(65001)` / `SetConsoleCP(65001)` before any I/O.
- CRLF-tolerant stdin parsing via `normalizar_linha_stdin` for cross-platform scripting.
- Parallel stdout/stderr capture on every remote command execution.
- Signal handling: SIGTERM/SIGINT via `ctrlc` + `signal_hook` — exit codes `130` (SIGINT) and `143` (SIGTERM).
- Sysexits.h exit codes (`EX_OK`, `EX_USAGE`, `EX_DATAERR`, `EX_NOINPUT`, `EX_CANTCREAT`, `EX_IOERR`, `EX_NOPERM`).
- Shell completions generated via `clap_complete` for Bash, Zsh, Fish, and PowerShell.
- Test suite: unit tests, E2E CLI (`assert_cmd` + `predicates` + `tempfile` + `serial_test`), property tests (`proptest`), snapshot tests (`insta`).
- Cross-platform builds: Linux (`x86_64-gnu`, `x86_64-musl`, `aarch64-gnu`, `aarch64-musl`), macOS (`x86_64`, `aarch64`, Universal), Windows (`x86_64-msvc`).
- MSRV pinned to Rust `1.85.0` via `rust-toolchain.toml`.
- Release automation via GitHub Actions (`release.yml`) with 8-target matrix and SHA256SUMS.

### Security
- Passwords wrapped in `SecretString` end-to-end; `Debug` impl redacts every secret.
- Config file receives `chmod 0o600` immediately after every write on Unix.
- `SSH_CLI_HOME` rejects any value containing `..` to prevent path traversal attacks.

[Unreleased]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/daniloaguiarbr/ssh-cli/releases/tag/v0.1.0
