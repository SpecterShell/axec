---
name: axec-list
description: List all tracked axec sessions with their status, UUID, name, and command info. Use when you need to see what sessions exist before interacting with them.
tools: Bash
---

# axec list — List Sessions

Show all tracked sessions with UUID, name, status, timestamps, and command line.

## Usage

```bash
axec list [OPTIONS]
```

Alias: `axec sessions`

## Options

| Option | Description |
|--------|-------------|
| `--json` | Emit structured JSON response with full session metadata |

## Examples

### List all sessions
```bash
axec list
```

### JSON output for programmatic use
```bash
axec list --json
```

## Session Metadata

Each session shows:
- **UUID** — unique identifier (use full or prefix with `--session`)
- **Name** — optional human-readable name (set via `axec run --name`)
- **Status** — `running` or `exited (code N)`
- **PID** — process ID
- **Command** — the command and arguments
- **Started at** — timestamp
- **Exited at** — timestamp (if exited)
- **Backend** — pty, pipe, or auto

## Best Practices

1. **Run `list` before starting new sessions** to avoid creating duplicates.
2. **Use `--json`** when you need to parse session metadata programmatically.
3. **Use session names** for easy identification — UUIDs are assigned automatically but names are optional.
