# axec Skills

Agent skills for **axec** — a daemon-backed CLI for persistent async command and REPL sessions.

These skills allow AI agents (Claude Code, etc.) to manage long-running processes, REPLs, and background jobs via axec.

## Available Skills

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

## Installation

```bash
# Build from source
cargo build --release

# The binary is at target/release/axec
```

## Quick Reference

```bash
# Typical agent workflow
axec run --name py python3              # Start Python REPL
axec input --session py --timeout 3 "print(42)"  # Send code, wait 3s
axec output --session py                # Read new output
axec kill --session py                  # Stop session
axec clean                              # Clean up
```

## Global Options

All commands accept `--json` for structured JSON output suitable for machine parsing.

## Session Selection

Anywhere `--session` is accepted, you can pass:
- Full UUID: `--session 550e8400-e29b-41d4-a716-446655440000`
- Unique UUID prefix: `--session 550e84`
- Session name: `--session py`
- Omit `--session` to use the latest session (in `cat`, `output`, `input`, `signal`)
