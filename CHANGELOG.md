# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial MVP scaffold: Cargo workspace, 8-target `rust-toolchain.toml`, `.cargo/config.toml`, `Cross.toml`, `deny.toml`, `.gitignore`, MIT `LICENSE`.
- CLI front-end via `clap` derive com subcomandos `vps {add,list,remove,edit,show,path}`, `connect <name>`, `exec`, `sudo-exec`, `scp`, `tunnel`.
- VPS registry stored as TOML at `$XDG_CONFIG_HOME/ssh-cli/config.toml` with automatic `chmod 0o600` on Unix (`PermissionsExt`).
- `VpsRegistro` model with password fields wrapped in `secrecy::SecretString` (Zeroize on Drop) and custom `Debug` that redacts every secret.
- Unicode-safe password masking (`12 first + 4 last` chars; `***` for strings with length ≤ 16).
- Deduplication on `vps add` (returns `VpsDuplicada` error if name exists).
- Schema versioning (`schema_version: u32`) and RFC 3339 timestamps (`added_at`).
- Path-traversal protection for `SSH_CLI_HOME` override.
- Platform init layer (`platform/{linux,macos,windows}.rs`) — Windows calls `SetConsoleOutputCP(65001)` / `SetConsoleCP(65001)` before any I/O.
- `normalizar_linha_stdin` strips `\r` / `\n` tails (CRLF-tolerant on Windows).
- Execução remota de comandos via SSH com captura separada de `stdout` e `stderr`.
- Subcomando `sudo-exec` para execução com elevação via `sudo`.
- Transferência de arquivos com `scp upload` e `scp download`.
- Port forwarding local via subcomando `tunnel`.
- i18n scaffolding via `rust-i18n` with auto-detection (`sys-locale`) and CLI override `--lang`. Locales `en-US` and `pt-BR` wired up.
- Test suite: testes unitários + E2E CLI (`assert_cmd` + `predicates` + `tempfile` + `serial_test`) + doctest.
- README bilingual EN/PT with badges, hero, Quick Start, VPS CRUD table, macOS Gatekeeper notes.
- CHANGELOG following Keep a Changelog.

### Security
- Passwords are `SecretString` end-to-end; `Debug` prints `<redacted>`.
- Config file receives `chmod 600` immediately after every write.
- `SSH_CLI_HOME` rejects any value containing `..` to prevent path traversal.

## [0.2.1] - 2026-04-16

### Fixed
- Pin `elliptic-curve = "=0.14.0-rc.30"` to fix `cargo install ssh-cli` failure caused by incompatible `elliptic-curve 0.14.0-rc.31+` being resolved for `p256/p384/p521 0.14.0-rc.8`

## [0.2.0] - 2026-04-15

### Added
- Fix sudo-exec stdin password piping with `printf '%s\n'`
- Runtime overrides: --password, --sudo-password, --timeout flags on exec/sudo-exec/scp/tunnel
- LLM-friendly camelCase aliases (--sudoPassword, --suPassword)

## [0.1.0] - 2026-04-14

Initial release.

[Unreleased]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/daniloaguiarbr/ssh-cli/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/daniloaguiarbr/ssh-cli/releases/tag/v0.1.0
