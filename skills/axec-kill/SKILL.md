---
name: axec-kill
description: Force-kill an axec session or all sessions. Use when you need to stop a running process. For graceful shutdown, consider axec-signal with SIGINT first.
tools: Bash
---

# axec kill — Kill Sessions

Force-kill a specific session or all running sessions.

## Usage

```bash
axec kill --session UUID|NAME
axec kill --all
```

Alias: `axec terminate`

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | Kill a specific session (mutually exclusive with `--all`) |
| `--all` | Kill all running sessions (mutually exclusive with `--session`) |
| `--json` | Emit structured JSON response |

One of `--session` or `--all` is **required**.

## Examples

### Kill a specific session
```bash
axec kill --session py
```

### Kill all sessions
```bash
axec kill --all
```

### Graceful shutdown pattern (signal first, then kill)
```bash
axec signal --session py SIGINT
# Wait a moment for graceful shutdown
axec kill --session py
```

## Related

- **`axec signal`** — Send a specific OS signal (SIGINT, SIGTERM, etc.) for graceful shutdown.
- **`axec clean`** — Remove exited sessions and their on-disk state after killing.

## Best Practices

1. **Prefer `axec signal --session NAME SIGINT` first** for graceful shutdown — many programs handle SIGINT cleanly.
2. **Use `axec kill`** when a process is unresponsive or you need immediate termination.
3. **Follow up with `axec clean`** to remove the exited session's state from disk.
