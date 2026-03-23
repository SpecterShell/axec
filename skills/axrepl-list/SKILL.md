---
name: axrepl-list
description: List known REPL sessions with their driver info. Use when you need to see which REPL sessions exist and their status.
tools: Bash
---

# axrepl list — List REPL Sessions

Show all tracked REPL sessions with UUID, name, driver, status, and command. Only sessions with a recognized REPL driver are shown (filters out non-REPL axec sessions).

## Usage

```bash
axrepl list [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `--json` | Emit structured JSON response |

## Examples

### List all REPL sessions
```bash
axrepl list
```

### JSON output for programmatic use
```bash
axrepl list --json
```

## Session Metadata

Each session shows:
- **UUID** — unique identifier
- **Name** — optional human-readable name
- **Driver** — `python`, `node`, `bash`, or `zsh`
- **Status** — `running` or `exited(N)`
- **Command** — the REPL command

## Best Practices

1. **Use `axrepl list`** instead of `axec list` when working only with REPLs — it filters out non-REPL sessions.
2. **Use `--json`** when you need to parse session metadata programmatically.

## Related

- **`axec list`** — List all sessions (including non-REPL).
