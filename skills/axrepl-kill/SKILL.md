---
name: axrepl-kill
description: Force-kill a running REPL session or all sessions. Use when you need to stop REPL processes managed by axrepl.
tools: Bash
---

# axrepl kill — Kill REPL Sessions

Force-kill a specific REPL session or all running sessions.

## Usage

```bash
axrepl kill --session UUID|NAME
axrepl kill --all
```

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | Kill a specific session (mutually exclusive with `--all`) |
| `--all` | Kill all running sessions (mutually exclusive with `--session`) |
| `--json` | Emit structured JSON response |

One of `--session` or `--all` is **required**.

## Examples

### Kill a specific REPL session
```bash
axrepl kill --session py
```

### Kill all sessions
```bash
axrepl kill --all
```

## Best Practices

1. **Follow up with `axrepl clean`** to remove the exited session's on-disk state.
2. **Use `axrepl kill`** interchangeably with `axec kill` — both send kill signals to the same daemon-managed sessions.

## Related

- **`axrepl clean`** — Remove exited REPL sessions and their on-disk state.
- **`axec kill`** — Same kill functionality (sessions are shared between axec and axrepl).
