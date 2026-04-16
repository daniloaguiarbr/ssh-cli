# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] - 2026-04-16

### Added
- `vps remove` now requires an explicit confirmation step. In TTY mode the command prompts `Remove VPS 'name'? (y/N):` (or `(s/N):` in Portuguese) before deleting. The new `--yes` / `-y` flag bypasses the prompt for scripted use. Non-interactive invocations without `--yes` fail fast with a localized error asking for the flag — preventing accidental destruction in CI pipelines and pipes.
- 3 new i18n variants (`ConfirmarRemocaoVps`, `RemocaoCancelada`, `RemoveExigeYesEmNaoInterativo`) in both English and Portuguese.
- 2 new public functions in `output.rs`: `stdin_e_tty()` wrapping `IsTerminal`, and `ler_confirmacao` as a pure testable helper accepting `&mut impl BufRead + &mut impl Write` with 8 unit tests covering acceptance (`s`/`sim`/`y`/`yes` case-insensitive), rejection (`n`/empty/EOF/arbitrary text), and trimming.
- 6 new regression tests: 2 E2E in `tests/e2e_cli.rs` (--yes short/long flag, non-interactive error), 4 in `tests/i18n_integration.rs` (confirmation prompt bilingual, cancellation bilingual, non-interactive error bilingual).

### Changed
- CI and release workflows upgraded from `actions/checkout@v4` / `actions/upload-artifact@v4` / `actions/download-artifact@v4` to their `@v5` counterparts (Node.js 24 native). Prevents forced migration warnings starting June 2026 and prepares for the October 2026 runner removal.
- 21 action reference updates across `.github/workflows/ci.yml` (12 `checkout`) and `.github/workflows/release.yml` (4 `checkout` + 2 `upload-artifact` + 3 `download-artifact`).

### Fixed
- `ssh-cli vps remove <name>` no longer silently removes the VPS without confirmation in interactive terminals.

## [0.3.1] - 2026-04-16

### Fixed
- `--output-format json vps list` with an empty registry now returns a valid JSON array (`[]`) instead of the localized text "Nenhum VPS cadastrado." The global `--output-format` flag was being silently ignored in `executar_comando_vps` (the `_formato` parameter was prefixed with underscore). This fix restores the LLM-automation contract that `--output-format json` always yields parseable JSON on stdout, regardless of whether the list is empty or populated.
- `--lang en` and `SSH_CLI_LANG=en` now properly force English in `ErroSshCli` error messages routed to stderr. Previously all `thiserror` `#[error("...")]` attributes were hardcoded in Portuguese, bypassing the i18n layer entirely. Added `ErroSshCli::mensagem_i18n()` method that maps every domain error variant to the corresponding `Mensagem` enum and consults `i18n::t()` at display time. `imprimir_erro_dominio` now calls this method instead of `Display`. Layer 1 (`--lang` flag) > Layer 2 (`SSH_CLI_LANG` env) > Layer 3 (`sys_locale`) > Layer 4 (English) precedence is now correctly applied to error paths.

### Added
- 6 new end-to-end regression tests: 2 for JSON-output contract (`testa_vps_list_vazia_com_output_format_json_retorna_array_vazio`, `testa_vps_list_com_uma_vps_output_format_json_mascara_senha`) and 4 for i18n override precedence (`test_lang_en_override_forca_ingles_em_erro_vps_nao_encontrada`, `test_lang_pt_override_forca_portugues_em_erro_vps_nao_encontrada`, `test_ssh_cli_lang_env_override_forca_ingles_em_erro`, `test_lang_flag_tem_precedencia_sobre_env`).

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

[Unreleased]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/daniloaguiarbr/ssh-cli/releases/tag/v0.1.0
