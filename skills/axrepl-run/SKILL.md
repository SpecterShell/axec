---
name: axrepl-run
description: Start a REPL session with driver-aware completion tracking. Use when you need to launch a Python, Node, Bash, or Zsh REPL that supports completion-aware input via axrepl input.
tools: Bash
---

# axrepl run — Start a REPL Session

Start a REPL process in a daemon-backed session with automatic driver detection. The driver determines how `axrepl input` wraps scripts to detect completion.

## Usage

```bash
axrepl run [OPTIONS] <CMD> [ARGS...]
```

## Options

| Option | Description |
|--------|-------------|
| `--name NAME` | Human-readable session name for easy reference later |
| `--driver DRIVER` | Override REPL driver (`python`, `node`, `bash`, `zsh`) when auto-detection fails |
| `--cwd DIR` | Working directory for the spawned REPL |
| `--env K=V` | Set environment variables (repeatable) |
| `--json` | Emit structured JSON response |

## Supported Drivers

- **python** — Detected for `python`, `python3`, `py`
- **node** — Detected for `node`, `nodejs`
- **bash** — Detected for `bash`
- **zsh** — Detected for `zsh`

The driver is inferred from the command name. Use `--driver` when the command path is ambiguous (e.g., a custom wrapper script).

## Examples

### Start a Python REPL
```bash
axrepl run --name py python3
```

### Start a Node REPL
```bash
axrepl run --name js node
```

### Start with explicit driver
```bash
axrepl run --name custom --driver python ./my-python-wrapper
```

### Start with custom working directory
```bash
axrepl run --name dev --cwd /path/to/project python3
```

## Best Practices

1. **Always use `--name`** for sessions you plan to reuse — it makes later `--session` references readable.
2. **Use `--driver`** when the command name doesn't match a known REPL (e.g., custom wrappers or virtualenv paths).
3. **Prefer `axrepl` over `axec`** for REPL workflows — it handles completion detection automatically via `axrepl input`.
4. **Use `--json`** when parsing the output programmatically — it returns the session UUID, name, and detected driver.

## Related

- **`axrepl input`** — Send completion-aware scripts to the REPL session.
- **`axec run`** — For non-REPL commands or when you need `--timeout`/`--stopword`/`--backend` options.
