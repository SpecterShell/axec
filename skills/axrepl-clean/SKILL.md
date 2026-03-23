---
name: axrepl-clean
description: Remove exited REPL sessions and their on-disk state. Use after killing REPL sessions or when finished with completed REPL jobs.
tools: Bash
---

# axrepl clean — Clean Up REPL Sessions

Remove all exited sessions and their on-disk state (metadata, output logs, REPL driver info).

## Usage

```bash
axrepl clean [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `--json` | Emit structured JSON response |

Only **exited** sessions are cleaned. Running sessions are untouched.

## Examples

### Clean up all exited sessions
```bash
axrepl clean
```

### Full cleanup workflow
```bash
axrepl kill --all
axrepl clean
```

## Best Practices

1. **Run `clean` periodically** to free disk space from accumulated session logs.
2. **Kill sessions first** if you want to clean everything: `axrepl kill --all && axrepl clean`.
3. **Use `axrepl clean`** interchangeably with `axec clean` — both clean the same session store.

## Related

- **`axrepl kill`** — Kill running REPL sessions before cleaning.
- **`axec clean`** — Same clean functionality (sessions are shared between axec and axrepl).
