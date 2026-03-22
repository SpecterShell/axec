---
name: axec-input
description: Send input to a running axec session. Use when you need to execute code in a REPL, send commands to a shell, or interact with any running session.
tools: Bash
---

# axec input — Send Input to a Session

Send text input to a running session's stdin, optionally streaming output back with a timeout or stopword.

## Usage

```bash
axec input [OPTIONS] <TEXT>
axec input [OPTIONS] - <<'EOF'
multi-line input here
EOF
```

## Options

| Option | Description |
|--------|-------------|
| `--session UUID\|NAME` | Target session (defaults to latest if omitted) |
| `--timeout SECONDS` | Stream output for N seconds after sending input |
| `--stopword REGEX` | Stream output until regex matches |
| `--terminate` | Kill the session after timeout/stopword triggers |
| `--json` | Emit structured JSON response |

## Examples

### Send a single line to a Python session
```bash
axec input --session py --timeout 3 "print(40 + 2)"
```

### Send to the latest session (no --session)
```bash
axec input --timeout 3 "print('hello')"
```

### Multi-line input with heredoc
```bash
axec input --session py - <<'EOF'
x = 40
y = 2
print(x + y)
EOF
```

### Piped input
```bash
printf 'line1\nline2\n' | axec input --session shell -
```

### Wait for a specific pattern in output
```bash
axec input --session py --stopword 'ready|done' "run_job()"
```

### Send and terminate after output
```bash
axec input --session py --timeout 5 --terminate "long_task()"
```

## Best Practices

1. **Always use `--timeout` or `--stopword`** to capture the response — otherwise the command returns immediately with no output.
2. **Use `--timeout`** when you know roughly how long the command takes.
3. **Use `--stopword`** with a regex matching your expected output pattern (e.g., a prompt like `>>>` or a result pattern).
4. **Use heredoc (`-`)** for multi-line code blocks — remember the trailing newline is important for REPLs.
5. **Use `--json`** for machine-parseable output when building automation.
6. If `--session` is omitted, the **latest started session** is used automatically.
