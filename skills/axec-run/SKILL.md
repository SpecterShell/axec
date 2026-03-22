---
name: axec-run
description: Start a persistent background session with axec. Use when you need to launch a long-running command, REPL (Python, Node, etc.), shell, or background job that persists across CLI calls.
tools: Bash
---

# axec run — Start a Persistent Session

Start a command in a daemon-backed background session. The process persists after the CLI returns, allowing later input/output interaction.

## Usage

```bash
axec run [OPTIONS] <CMD> [ARGS...]
```

## Options

| Option | Description |
|--------|-------------|
| `--name NAME` | Human-readable session name for easy reference later |
| `--timeout SECONDS` | Stream early output for N seconds before returning |
| `--stopword REGEX` | Stream output until regex matches, then return |
| `--terminate` | Kill the session after timeout/stopword triggers |
| `--backend pty\|pipe\|auto` | I/O backend (default: `pty`) |
| `--cwd DIR` | Working directory for the command |
| `--env K=V` | Set environment variables (repeatable) |
| `--json` | Emit structured JSON response |

## Backend Modes

- **pty** (default): Merged stdout/stderr, fully interactive terminal. Best for REPLs and shells.
- **pipe**: Separate stdout/stderr streams. Best for non-interactive commands where you need split output.
- **auto**: Platform heuristics choose between pty and pipe.

## Examples

### Start a Python REPL
```bash
axec run --name py python3
```

### Start a shell session
```bash
axec run --name shell bash
```

### Run a command with split stdout/stderr
```bash
axec run --name build --backend pipe sh -c 'echo out; echo err >&2'
```

### Start with timeout to capture early output
```bash
axec run --name server --timeout 5 node server.js
```

### Start with custom working directory and env vars
```bash
axec run --name dev --cwd /path/to/project --env NODE_ENV=development node app.js
```

### One-shot command with auto backend
```bash
axec run --name check --backend auto sh -c 'echo ok; echo warn >&2; sleep 1'
```

## Best Practices

1. **Always use `--name`** for sessions you plan to reuse — it makes later `--session` references readable.
2. **Use `--backend pipe`** for non-interactive commands when you need separate stdout/stderr.
3. **Use `--timeout`** when you want to verify the process started correctly before moving on.
4. **Use `--json`** when parsing the output programmatically — it returns the session UUID and metadata.
5. **Check `axec list` first** to avoid creating duplicate sessions with the same name.
