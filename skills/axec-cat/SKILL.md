---
name: axec-cat
description: Print full output history from an axec session, optionally following live output. Use when you need the complete log or want to stream output in real time.
tools: Bash
---

# axec cat — Full Output History

Print the full recorded output of a session. Optionally follow live output as it arrives.

## Usage

```bash
axec cat [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | Target session (defaults to latest if omitted) |
| `--follow` | After printing history, keep streaming live output |
| `--stderr` | Print stderr instead of stdout (only with `pipe` backend) |
| `--json` | Emit structured JSON response |

## Examples

### Print full stdout history
```bash
axec cat --session build
```

### Print stderr (pipe backend only)
```bash
axec cat --session build --stderr
```

### Follow live output
```bash
axec cat --session server --follow
```

### From the latest session
```bash
axec cat
```

## cat vs output

| | `axec cat` | `axec output` |
|---|---|---|
| **Scope** | Full history from session start | Only unread since last call |
| **Cursor** | Does not advance read cursor | Advances read cursor |
| **Use case** | Review complete log, stream live | Incremental polling |

## Best Practices

1. **Use `cat`** to review the complete output of a session.
2. **Use `cat --follow`** to monitor a long-running process in real time.
3. **Use `cat --stderr`** only with `--backend pipe` sessions — pty backend merges stderr into stdout.
4. **Use `output`** instead when you only want new, unread content.
