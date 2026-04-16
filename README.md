# ssh-cli

[![crates.io](https://img.shields.io/crates/v/ssh-cli.svg)](https://crates.io/crates/ssh-cli)
[![docs.rs](https://docs.rs/ssh-cli/badge.svg)](https://docs.rs/ssh-cli)
[![CI](https://github.com/comandoaguiar/ssh-cli/workflows/CI/badge.svg)](https://github.com/comandoaguiar/ssh-cli/actions)
[![MSRV](https://img.shields.io/badge/MSRV-1.85.0-blue)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)
[![license](https://img.shields.io/crates/l/ssh-cli.svg)](LICENSE)

---

## English

> Give any LLM the power to operate remote servers via SSH — in a single, memory-safe binary.

### What is it?
- Single static binary — zero runtime dependencies, no Node.js, no Python.
- Full Rust SSH stack — `russh` + `aws-lc-rs`, no C bindings, memory-safe end to end.
- Cold start under 100 ms on any supported platform.
- Credentials zeroized on drop via `secrecy::SecretString` — never linger in RAM.
- Parallel stdout/stderr capture on every remote command execution.
- 2 locales built in — `en-US` and `pt-BR`, auto-detected from system locale.

### Why ssh-cli?
- Eliminates Node.js-based SSH wrappers and their runtime overhead.
- Ships as one file — copy and run, zero install step or package manager required.
- ZERO `.env` files — all credentials managed exclusively via explicit CLI subcommands.
- Sysexits.h exit codes enable reliable scripting and LLM error classification.

### Quick Start

```bash
# Install
cargo install ssh-cli

# Register a VPS
ssh-cli vps add \
  --name prod \
  --host prod.example.com \
  --port 22 \
  --user admin \
  --password 's3cret'

# Select the active VPS
ssh-cli connect prod

# Execute a remote command
ssh-cli exec prod "hostname"
```

### Command Reference

| Command | Purpose |
|---|---|
| `ssh-cli vps add --name X --host Y …` | Add a VPS record (deduplicated by name) |
| `ssh-cli vps list [--json]` | List registered VPSs (passwords masked) |
| `ssh-cli vps show <name> [--json]` | Show one VPS (passwords masked) |
| `ssh-cli vps edit <name> --host Z` | Edit any field of an existing VPS |
| `ssh-cli vps remove <name>` | Remove a VPS from the registry |
| `ssh-cli vps path` | Print the path of `config.toml` |
| `ssh-cli connect <name>` | Mark a VPS as active for subsequent commands |
| `ssh-cli exec <vps> <cmd> [--json]` | Execute a command on a VPS via SSH |
| `ssh-cli sudo-exec <vps> <cmd> [--json]` | Execute a command with `sudo` on a VPS |
| `ssh-cli scp upload <vps> <local> <remote>` | Upload a file via SCP |
| `ssh-cli scp download <vps> <remote> <local>` | Download a file via SCP |
| `ssh-cli tunnel <vps> <local-port> <remote-host> <remote-port>` | Open SSH local port forward |
| `ssh-cli health-check [<vps>]` | Verify SSH connectivity (uses active VPS if omitted) |
| `ssh-cli completions <shell>` | Print shell completions to stdout |

### Global Flags

| Flag | Short | Purpose |
|---|---|---|
| `--lang <LOCALE>` | | Force language (`en-US`, `pt-BR`) |
| `--verbose` | `-v` | Increase log verbosity to `debug` |
| `--quiet` | `-q` | Suppress non-JSON output (`error` only) |
| `--config-dir <DIR>` | | Override the configuration directory |
| `--no-color` | | Disable ANSI colors in output |
| `--output-format <FORMAT>` | | Output format: `text` (default) or `json` |

### Environment Variables

| Variable | Description | Example |
|---|---|---|
| `SSH_CLI_HOME` | Override the base configuration directory | `/tmp/ssh-cli-test` |
| `SSH_CLI_LANG` | Override the detected locale | `pt-BR` |
| `NO_COLOR` | Disable ANSI colors (any non-empty value) | `1` |
| `CLICOLOR_FORCE` | Force ANSI colors even when not a TTY | `1` |

### Exit Codes

| Code | Constant | Meaning |
|---|---|---|
| `0` | `EX_OK` | Success |
| `1` | `EX_GENERAL` | Generic runtime error |
| `64` | `EX_USAGE` | Incorrect CLI usage or invalid argument |
| `65` | `EX_DATAERR` | Invalid input data (JSON/TOML parse error) |
| `66` | `EX_NOINPUT` | VPS or file not found |
| `73` | `EX_CANTCREAT` | Cannot create output (config write failed) |
| `74` | `EX_IOERR` | I/O or SSH connection error |
| `77` | `EX_NOPERM` | SSH authentication rejected |
| `130` | `EX_SIGINT` | Terminated by SIGINT (Ctrl+C) |
| `143` | `EX_SIGTERM` | Terminated by SIGTERM |

### Shell Completions
- Bash: `ssh-cli completions bash > ~/.local/share/bash-completion/completions/ssh-cli`
- Zsh: `ssh-cli completions zsh > ~/.zfunc/_ssh-cli`
- Fish: `ssh-cli completions fish > ~/.config/fish/completions/ssh-cli.fish`
- PowerShell: `ssh-cli completions powershell >> $PROFILE`

### Troubleshooting
- macOS Gatekeeper blocks the binary: run `xattr -d com.apple.quarantine /path/to/ssh-cli`
- Alpine Linux or musl target: build with `--features musl-allocator`
- Permission denied on `config.toml`: run `chmod 600 ~/.config/ssh-cli/config.toml`

### License

MIT — see [LICENSE](LICENSE).

---

## Português (Brasil)

> Dê a qualquer LLM o poder de operar servidores remotos via SSH — em um único binário, seguro por design.

### O que é?
- Binário único estático — zero dependência de runtime, sem Node.js, sem Python.
- Stack SSH completa em Rust — `russh` + `aws-lc-rs`, sem bindings C, memory-safe ponta a ponta.
- Cold start abaixo de 100 ms em qualquer plataforma suportada.
- Credenciais zeradas da memória via `secrecy::SecretString` — nunca ficam na RAM.
- Captura paralela de stdout/stderr em toda execução remota.
- 2 locales prontos — `en-US` e `pt-BR`, detectados automaticamente pelo locale do sistema.

### Por que ssh-cli?
- Elimina wrappers SSH baseados em Node.js e seu overhead de runtime.
- Um único arquivo — copie e execute, sem passo de instalação ou gerenciador de pacotes.
- ZERO arquivos `.env` — todas as credenciais gerenciadas exclusivamente via subcomandos CLI.
- Exit codes do sysexits.h permitem scripting confiável e classificação de erros por LLMs.

### Início rápido

```bash
# Instalar
cargo install ssh-cli

# Registrar uma VPS
ssh-cli vps add \
  --name producao \
  --host producao.exemplo.com \
  --port 22 \
  --user admin \
  --password 's3gred0'

# Selecionar a VPS ativa
ssh-cli connect producao

# Executar comando remoto
ssh-cli exec producao "hostname"
```

### Referência de comandos

| Comando | Propósito |
|---|---|
| `ssh-cli vps add --name X --host Y …` | Adiciona uma VPS (deduplicação por nome) |
| `ssh-cli vps list [--json]` | Lista VPSs registradas (senhas mascaradas) |
| `ssh-cli vps show <nome> [--json]` | Exibe uma VPS (senhas mascaradas) |
| `ssh-cli vps edit <nome> --host Z` | Edita campos de uma VPS existente |
| `ssh-cli vps remove <nome>` | Remove uma VPS do registro |
| `ssh-cli vps path` | Mostra o caminho do `config.toml` |
| `ssh-cli connect <nome>` | Define a VPS ativa para os próximos comandos |
| `ssh-cli exec <vps> <cmd> [--json]` | Executa comando em uma VPS via SSH |
| `ssh-cli sudo-exec <vps> <cmd> [--json]` | Executa comando com `sudo` em uma VPS |
| `ssh-cli scp upload <vps> <local> <remoto>` | Envia arquivo via SCP |
| `ssh-cli scp download <vps> <remoto> <local>` | Baixa arquivo via SCP |
| `ssh-cli tunnel <vps> <porta-local> <host-remoto> <porta-remota>` | Cria port-forward local via SSH |
| `ssh-cli health-check [<vps>]` | Verifica conectividade SSH (usa VPS ativa se omitida) |
| `ssh-cli completions <shell>` | Imprime completions de shell no stdout |

### Flags globais

| Flag | Abreviação | Propósito |
|---|---|---|
| `--lang <LOCALE>` | | Força o idioma (`en-US`, `pt-BR`) |
| `--verbose` | `-v` | Aumenta verbosidade dos logs para `debug` |
| `--quiet` | `-q` | Suprime output não-JSON (somente `error`) |
| `--config-dir <DIR>` | | Substitui o diretório de configuração |
| `--no-color` | | Desativa cores ANSI no output |
| `--output-format <FORMATO>` | | Formato de saída: `text` (padrão) ou `json` |

### Variáveis de ambiente

| Variável | Descrição | Exemplo |
|---|---|---|
| `SSH_CLI_HOME` | Substitui o diretório base de configuração | `/tmp/ssh-cli-teste` |
| `SSH_CLI_LANG` | Substitui o locale detectado | `pt-BR` |
| `NO_COLOR` | Desativa cores ANSI (qualquer valor não-vazio) | `1` |
| `CLICOLOR_FORCE` | Força cores ANSI mesmo sem TTY | `1` |

### Exit codes

| Código | Constante | Significado |
|---|---|---|
| `0` | `EX_OK` | Sucesso |
| `1` | `EX_GENERAL` | Erro genérico de runtime |
| `64` | `EX_USAGE` | Uso incorreto da CLI ou argumento inválido |
| `65` | `EX_DATAERR` | Dados de entrada inválidos (JSON/TOML) |
| `66` | `EX_NOINPUT` | VPS ou arquivo não encontrado |
| `73` | `EX_CANTCREAT` | Falha ao criar saída (config não gravou) |
| `74` | `EX_IOERR` | Erro de I/O ou conexão SSH |
| `77` | `EX_NOPERM` | Autenticação SSH rejeitada |
| `130` | `EX_SIGINT` | Terminado por SIGINT (Ctrl+C) |
| `143` | `EX_SIGTERM` | Terminado por SIGTERM |

### Completions de shell
- Bash: `ssh-cli completions bash > ~/.local/share/bash-completion/completions/ssh-cli`
- Zsh: `ssh-cli completions zsh > ~/.zfunc/_ssh-cli`
- Fish: `ssh-cli completions fish > ~/.config/fish/completions/ssh-cli.fish`
- PowerShell: `ssh-cli completions powershell >> $PROFILE`

### Solução de problemas
- macOS Gatekeeper bloqueia o binário: execute `xattr -d com.apple.quarantine /caminho/para/ssh-cli`
- Alpine Linux ou target musl: compile com `--features musl-allocator`
- Permissão negada no `config.toml`: execute `chmod 600 ~/.config/ssh-cli/config.toml`

### Licença

MIT — veja [LICENSE](LICENSE).
