# axec Skills

Agent skills for **axec** — a daemon-backed CLI for persistent async command and REPL sessions.

These skills allow AI agents (Claude Code, etc.) to manage long-running processes, REPLs, and background jobs via axec and axrepl.

## Available Skills

### axec — General Session Management

| Skill | Description | Trigger |
|-------|-------------|---------|
| [axec-run](axec-run/SKILL.md) | Start a persistent background session | Need to launch a command, REPL, or shell that persists |
| [axec-input](axec-input/SKILL.md) | Send input to a running session | Need to execute code or commands in an existing session |
| [axec-output](axec-output/SKILL.md) | Read unread output from a session | Need to check new output since last read |
| [axec-cat](axec-cat/SKILL.md) | Print full output history or follow live | Need complete output log or live streaming |
| [axec-list](axec-list/SKILL.md) | List all tracked sessions | Need to see what sessions exist and their status |
| [axec-kill](axec-kill/SKILL.md) | Kill a session or all sessions | Need to stop running processes |
| [axec-clean](axec-clean/SKILL.md) | Remove exited sessions and on-disk state | Need to clean up finished sessions |
| [axec-attach](axec-attach/SKILL.md) | Interactively attach to a running session | Need interactive terminal access to a session |

### axrepl — Completion-Aware REPL Management

| Skill | Description | Trigger |
|-------|-------------|---------|
| [axrepl-run](axrepl-run/SKILL.md) | Start a REPL session with driver detection | Need to launch a Python, Node, Bash, or Zsh REPL |
| [axrepl-input](axrepl-input/SKILL.md) | Send a script with completion-aware execution | Need to run code in a REPL and reliably capture full output |
| [axrepl-list](axrepl-list/SKILL.md) | List known REPL sessions | Need to see REPL sessions and their drivers |
| [axrepl-kill](axrepl-kill/SKILL.md) | Kill REPL sessions | Need to stop running REPL processes |
| [axrepl-clean](axrepl-clean/SKILL.md) | Remove exited REPL sessions | Need to clean up finished REPL sessions |

## When to Use axrepl vs axec

- **Use `axrepl`** for interactive REPL sessions (Python, Node, Bash, Zsh) where you want automatic completion detection — no need to guess `--timeout` or `--stopword` values.
- **Use `axec`** for general commands, non-REPL processes, or when you need features like `--backend pipe`, `output`, `cat`, `attach`, or `signal`.

Both tools share the same daemon and session store — sessions created by `axrepl run` are visible to `axec list` and vice versa.

## Installation

```bash
# Build from source
cargo build --release

# Binaries are at target/release/axec and target/release/axrepl
```

## Quick Reference

```bash
# REPL workflow with axrepl (recommended for REPLs)
axrepl run --name py python3                # Start Python REPL
axrepl input --session py "print(42)"       # Send code, auto-detects completion
axrepl input --session py - <<'EOF'         # Multi-line input
for i in range(5):
    print(i)
EOF
axrepl list                                 # List REPL sessions
axrepl kill --session py                    # Stop session
axrepl clean                                # Clean up

# General workflow with axec
axec run --name py python3                  # Start Python REPL
axec input --session py --timeout 3 "print(42)"  # Send code, wait 3s
axec output --session py                    # Read new output
axec kill --session py                      # Stop session
axec clean                                  # Clean up
```

## Global Options

All commands accept `--json` for structured JSON output suitable for machine parsing.

## Session Selection

Anywhere `--session` is accepted, you can pass:
- Full UUID: `--session 550e8400-e29b-41d4-a716-446655440000`
- Unique UUID prefix: `--session 550e84`
- Session name: `--session py`
- Omit `--session` to use the latest session (in `axec`: `cat`, `output`, `input`, `signal`; in `axrepl`: `input`)
