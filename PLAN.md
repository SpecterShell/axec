# axec - Async Command Execution CLI

## Context

The goal is to build a cross-platform CLI tool that lets an AI model (or user) manage long-running REPL sessions and commands asynchronously. This solves three problems: (1) CLI commands are stateless/one-shot — context is lost after each run, (2) MCP servers can't cover all language libraries, and (3) MCP exports too many functions consuming LLM context. With `axec`, a model can start a Python/Node/bash REPL, send code to it over time, and monitor output — all through simple CLI commands.

## Architecture

**Daemon + CLI client** over Unix socket (Linux/macOS) / named pipe (Windows).

- The daemon auto-starts on first CLI command and auto-stops after idle timeout
- Each session wraps a child process in a PTY (via `portable-pty`) so interactive REPLs work naturally
- Output is captured to both an in-memory ring buffer (fast streaming) and append-only log files (stdout.log + stderr.log for full history)
- Wire protocol: length-delimited JSON frames over the socket

**Tech stack**: Rust, tokio, clap (derive), serde, portable-pty, uuid, dirs, tracing, thiserror, rust-i18n

## Commands

| Command | Behavior |
|---|---|
| `axec run [--name NAME] [--timeout N] [--terminate] [--cwd DIR] [--env K=V]... <cmd> [args]` | Spawn process, return UUID (and name if given), detach immediately |
| `axec run --timeout N <cmd>` | Spawn, stream output for N seconds, exit 0 (process keeps running) |
| `axec run --terminate <cmd>` | Spawn, wait for process to finish, pass exit code |
| `axec run --terminate --timeout N <cmd>` | Spawn, stream for N seconds, kill on timeout (exit 124) |
| `axec cat --session UUID\|NAME [--follow] [--stderr]` | Print stdout (or stderr) history; `--follow` streams live |
| `axec list` / `axec sessions` | List sessions with status |
| `axec input --session UUID\|NAME [--timeout N] [--terminate] <text>` | Send text to session's stdin |
| `axec input --session UUID\|NAME --timeout N - < file` | Send file contents as input, stream output |
| `axec signal --session UUID\|NAME <SIGNAL>` | Send signal (SIGINT, SIGTERM, etc.) to session process |
| `axec kill --session UUID\|NAME` | Kill a session's process (SIGKILL) |
| `axec attach --session UUID\|NAME` | Interactively attach terminal to session (like tmux attach) |
| `axec clean` | Remove dead sessions |

**Global flags**: `--json` for machine-readable JSON output on all commands.

**Session identification**: `--session` accepts either a UUID or a `--name` string. Names must be unique among active sessions.

**Exit codes**: 0 on success/timeout-without-terminate, 124 on timeout+terminate (matches GNU `timeout`), process exit code on natural exit, 1 on error.

## Project Structure

```
axec/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, dispatch to client or daemon (--daemon flag)
│   ├── cli.rs               # clap derive structs for all commands
│   ├── protocol.rs          # Request/Response enums, SessionInfo, SessionStatus
│   ├── config.rs            # Constants (buffer size, idle timeout, etc.)
│   ├── paths.rs             # Platform-appropriate socket/data paths
│   ├── error.rs             # thiserror error types
│   ├── i18n.rs              # rust-i18n setup, locale detection
│   ├── client/
│   │   ├── mod.rs
│   │   ├── connection.rs    # Connect to daemon, auto-start, framed I/O
│   │   └── commands.rs      # Per-command client logic
│   ├── daemon/
│   │   ├── mod.rs           # Daemon entry point
│   │   ├── server.rs        # Socket listener, request router, streaming
│   │   ├── session.rs       # Session: PTY handle, output buffers, broadcast
│   │   ├── session_manager.rs # HashMap<Uuid, Session>, name→UUID index, CRUD
│   │   ├── output_buffer.rs # Ring buffer + file-backed log (stdout + stderr)
│   │   └── idle_monitor.rs  # Auto-shutdown on idle
│   └── platform/
│       ├── mod.rs           # cfg-gated re-exports
│       ├── unix.rs          # Unix socket, daemonize, signals
│       └── windows.rs       # Named pipe, detached process spawn
├── locales/
│   ├── en.yml               # English translations
│   └── zh-CN.yml            # Chinese translations
```

## Key Data Structures

- **Request** enum (tagged JSON): `Run { command, args, name, timeout, terminate, cwd, env }`, `Cat { session, follow, stderr }`, `List`, `Input { session, text, timeout, terminate }`, `Signal { session, signal }`, `Kill { session }`, `Attach { session }`, `Clean`, `Ping`
- **Response** enum (tagged JSON): `SessionCreated { uuid, name }`, `OutputChunk { data, stream: Stdout|Stderr, eof }`, `Finished { exit_code }`, `CatOutput { data }`, `SessionList { sessions }`, `Error { message }`
- **Session**: UUID, name (Option), command, status, PTY master writer, child handle, OutputBuffer (stdout + stderr), `broadcast::Sender` for live streaming, cwd, env overrides
- **OutputBuffer**: `VecDeque<u8>` ring (recent data) + append-only log file (full history), separate for stdout and stderr
- **SessionMeta** (on disk as `~/.axec/sessions/<uuid>/meta.json`): serializable session state

## Communication Patterns

- **One-shot** (cat, list, kill, signal, clean, run without timeout): send Request → receive one Response → close
- **Streaming** (run/input with --timeout, cat --follow, attach): send Request → receive multiple OutputChunk → receive Finished (or client disconnects) → close
- **Interactive** (attach): bidirectional — client forwards terminal input to daemon, daemon streams PTY output back. Client sets terminal to raw mode.

## Implementation Phases

### Phase 1: Skeleton and Protocol
Create `Cargo.toml`, `main.rs`, `cli.rs`, `protocol.rs`, `error.rs`, `paths.rs`, `config.rs`, `i18n.rs`, locale files. Goal: `axec --help` works with i18n.

### Phase 2: Daemon Core and IPC
Implement daemon socket listener, client connection with auto-start, framed JSON codec, Ping/Pong. Goal: `axec list` returns empty list after auto-starting daemon.

### Phase 3: Session Management and `run`
Implement Session with PTY, OutputBuffer (stdout+stderr), SessionManager with name index. Handle `Run` (detach, with `--name`, `--cwd`, `--env`), `Cat` (with `--stderr`), `List`, `Kill`, `Signal`, `Clean`. Goal: `axec run --name mypy python3` → UUID, `axec cat --session mypy` shows output.

### Phase 4: Streaming Output
Implement `--timeout` and `--terminate` flags with broadcast channel subscription and `tokio::time::timeout`. Implement `cat --follow`. Exit code 124 for timeout+terminate. Goal: `axec run --timeout 3 python3` streams output for 3 seconds.

### Phase 5: Input Command
Implement writing to PTY master, `--timeout` streaming after input, stdin redirection. Goal: `axec input --session mypy "print(42)"` works with Python REPL.

### Phase 6: Interactive Attach
Implement `axec attach` — set client terminal to raw mode, forward I/O bidirectionally to the PTY via daemon. Detach with escape sequence (e.g., `Ctrl+\`).

### Phase 7: JSON Output Mode and Daemon Lifecycle
- `--json` flag: all commands output structured JSON
- Idle monitor, PID file, stale daemon detection, graceful shutdown, stale session recovery on startup

### Phase 8: Windows Support
Named pipe transport, detached process spawn with `CREATE_NO_WINDOW`, test ConPTY behavior.

### Phase 9: Polish
Structured logging, table formatting for `list`, shell completions, config file support.

## Agentic Use Cases

### With Claude Code, Codex, Gemini CLI, etc. (CLI tool calling via Bash)

Claude Code can use `axec` through its Bash tool to maintain persistent REPL sessions across its entire conversation:

**Persistent data analysis environment**:
```bash
# Claude starts a named IPython session
axec run --name analysis ipython

# Later, loads data incrementally across turns
axec input --session analysis --timeout 10 "import pandas as pd; df = pd.read_csv('data.csv'); df.shape"

# Many turns later, still has the DataFrame in memory
axec input --session analysis --timeout 10 "df.describe()"
```

**Background long-running tasks** with JSON monitoring:
```bash
axec run --name train --json python train.py    # JSON output with UUID
# ... Claude works on other things ...
axec cat --session train --json                  # Check progress (machine-parseable)
axec list --json                                 # All sessions as JSON
```

**Interactive debugging**:
```bash
axec run --name debug python -m pdb script.py
axec input --session debug --timeout 5 "n"       # next line
axec input --session debug --timeout 5 "p x"     # print variable
axec signal --session debug SIGINT               # Ctrl+C to interrupt
```

### With Agentic Orchestrators (Claude Cowork, OpenClaw, custom agents)

**Shared environment across agents**:
```bash
# Agent A: setup
axec run --name shared-env bash
axec input --session shared-env --timeout 30 "pip install numpy pandas"

# Agent B: uses the same environment (receives name from orchestrator)
axec input --session shared-env --timeout 30 "python -c 'import numpy; print(numpy.__version__)'"
```

**Parallel async workloads** with JSON monitoring:
```bash
axec run --name north --json python analyze_north.py
axec run --name south --json python analyze_south.py
axec list --json     # Orchestrator parses JSON to check status
```

**Database exploration** with separate stderr:
```bash
axec run --name db psql -h localhost mydb
axec input --session db --timeout 10 "SELECT count(*) FROM users;"
axec cat --session db --stderr     # Check for SQL errors separately
```

### As a Claude Code Hook / Integration

- Pre-session hook: `axec run --name repl python3` auto-starts a REPL
- The model references `--session repl` throughout the conversation
- Post-session hook: `axec clean` tears down all sessions

## Verification

1. `cargo build` compiles without errors
2. `axec run --name test bash` returns a UUID; `axec list` shows it as Running
3. `axec input --session test --timeout 3 'echo hello'` prints "hello"
4. `axec cat --session test` shows full output; `axec cat --session test --stderr` shows stderr
5. `axec cat --session test --follow` live-streams new output
6. `axec run --timeout 5 python3 -c "import time; [print(i) or time.sleep(1) for i in range(10)]"` streams for 5 seconds, exits 0, process keeps running
7. `axec run --terminate --timeout 3 sleep 100` kills process after 3 seconds, exits 124
8. `axec signal --session test SIGINT` sends interrupt to process
9. `axec list --json` outputs valid JSON
10. `axec attach --session test` opens interactive session, `Ctrl+\` detaches
11. Daemon auto-starts on first command and auto-stops after idle
12. `LANG=zh_CN.UTF-8 axec --help` shows Chinese help text
13. `cargo test` passes unit and integration tests
