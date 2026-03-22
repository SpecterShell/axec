---
name: axec-attach
description: Interactively attach to a running axec session for direct terminal access. Use when you need full interactive control of a session (like tmux attach).
tools: Bash
---

# axec attach — Interactive Terminal Attach

Open an interactive terminal attached to a running session. This gives you direct keyboard input and live output, similar to `tmux attach` or `screen -r`.

## Usage

```bash
axec attach --session UUID|NAME
```

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | **Required.** The session to attach to |

## Detaching

Press **Ctrl+\\** to detach from the session without stopping it.

## Examples

### Attach to a named session
```bash
axec attach --session py
```

### Attach by UUID prefix
```bash
axec attach --session 2a40f9d2
```

## Limitations

- **`--session` is required** — unlike other commands, attach does not default to the latest session.
- This is an **interactive** command — it takes over the terminal until you detach.
- Best used for sessions started with the `pty` backend.

## Best Practices

1. **Use for debugging** — attach to inspect a session interactively, then detach to let it continue.
2. **Use `input`/`output` for automation** — attach is for human interaction, not scripted workflows.
3. **Remember Ctrl+\\** to detach — Ctrl+C sends SIGINT to the process inside the session.
