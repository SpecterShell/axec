---
name: axec-output
description: Print unread stdout from an axec session since the last read. Use when you need to check for new output without re-reading the full history.
tools: Bash
---

# axec output — Read Unread Output

Print only the stdout that has accumulated since the last `output` call for a session. This is an incremental read — each call advances the read cursor.

## Usage

```bash
axec output [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | Target session (defaults to latest if omitted) |
| `--json` | Emit structured JSON response |

## Examples

### Read new output from a named session
```bash
axec output --session py
```

### Read new output from the latest session
```bash
axec output
```

### Machine-readable output
```bash
axec output --session py --json
```

## How It Works

- Each session tracks a read cursor per client.
- `axec output` returns everything written to stdout since the cursor was last advanced.
- Calling `axec output` again immediately after returns empty if no new output has arrived.
- This is different from `axec cat`, which always prints the full history.

## Best Practices

1. **Use `output` for polling workflows** — call it periodically to get incremental results.
2. **Use `cat` instead** if you want the complete output history.
3. **Use `cat --follow`** if you want to stream live output continuously.
