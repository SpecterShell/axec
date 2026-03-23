---
name: axrepl-input
description: Send a script to a REPL session with completion-aware execution. Use when you need to execute code in a Python, Node, Bash, or Zsh REPL and reliably capture the full output without guessing timeouts.
tools: Bash
---

# axrepl input — Completion-Aware REPL Input

Send a script to a running REPL session. Unlike `axec input`, this command wraps the script with driver-specific code that injects a completion marker, so it knows exactly when the REPL has finished processing — no `--timeout` or `--stopword` needed.

## Usage

```bash
axrepl input [OPTIONS] <TEXT>
axrepl input [OPTIONS] - <<'EOF'
multi-line script here
EOF
```

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | Target session (defaults to latest REPL session if omitted) |
| `--driver DRIVER` | Override the detected REPL driver for this request |
| `--json` | Emit structured JSON response |

## How It Works

1. The script is wrapped in driver-specific code (Python `exec()`, Node `eval()`, shell `eval`) that catches errors and prints a unique completion marker after execution.
2. The wrapped input is sent to the session via the axec daemon.
3. Output is streamed until the completion marker is observed.
4. The marker and wrapper artifacts are stripped from the output, returning only the script's actual output.

## Examples

### Execute Python code
```bash
axrepl input --session py "print(40 + 2)"
```

### Multi-line Python script
```bash
axrepl input --session py - <<'EOF'
import math
for i in range(5):
    print(f"{i}: {math.factorial(i)}")
EOF
```

### Execute Node.js code
```bash
axrepl input --session js "console.log(Array.from({length: 5}, (_, i) => i * i))"
```

### Execute shell commands
```bash
axrepl input --session shell "ls -la /tmp"
```

### Read script from stdin
```bash
echo 'print("hello")' | axrepl input --session py -
```

### Use latest session (no --session)
```bash
axrepl input "print('hello')"
```

## Best Practices

1. **No timeout needed** — completion detection is automatic. The command returns as soon as the REPL finishes executing the script.
2. **Use heredoc (`-`)** for multi-line code blocks.
3. **Errors are captured** — if the script throws an exception, the traceback/error is included in the output and the command still returns successfully.
4. **Use `--driver`** to override the auto-detected driver if the session's driver wasn't recognized at `run` time.
5. **Use `--json`** for structured output that includes session UUID, driver, and the script output.
6. **Prefer `axrepl input` over `axec input`** for REPL sessions — it eliminates the guesswork of choosing `--timeout` or `--stopword` values.

## Related

- **`axrepl run`** — Start a REPL session with driver detection.
- **`axec input`** — Lower-level input with manual `--timeout`/`--stopword` for non-REPL sessions.
