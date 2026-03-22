---
name: axec-clean
description: Remove all exited axec sessions and their on-disk state (logs, metadata). Use after killing sessions or when finished with completed jobs.
tools: Bash
---

# axec clean — Clean Up Sessions

Remove all exited sessions and their on-disk state (metadata, stdout/stderr logs).

## Usage

```bash
axec clean [OPTIONS]
```

Alias: `axec clear`

## Options

| Option | Description |
|--------|-------------|
| `--json` | Emit structured JSON response |

## What Gets Removed

- Session metadata (`~/.axec/sessions/<uuid>/meta.json`)
- Output logs (`stdout.log`, `stderr.log`)
- The session directory itself

Only **exited** sessions are cleaned. Running sessions are untouched.

## Examples

### Clean up all exited sessions
```bash
axec clean
```

### Full cleanup workflow
```bash
axec kill --all        # Stop everything
axec clean             # Remove exited session state
```

## Best Practices

1. **Run `clean` periodically** to free disk space from accumulated session logs.
2. **Only exited sessions are removed** — you don't need to worry about cleaning active sessions.
3. **Kill sessions first** if you want to clean everything: `axec kill --all && axec clean`.
