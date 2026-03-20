# axec

`axec` is an async command execution CLI for long-running shells and REPLs.

It gives you a small daemon-backed interface for:

- starting detached commands and interactive shells
- sending more input to the same session later
- streaming live output or reading historical output
- listing, signaling, killing, cleaning, and attaching to sessions

The intended use case is persistent command execution for humans and agentic tools that need state across multiple CLI invocations.

## Status

Current implementation status:

- Unix runtime support is working and exercised
- daemon auto-start and idle shutdown are implemented
- PTY-backed sessions are implemented
- `run`, `exec`, `list`, `cat`, `input`, `signal`, `kill`, `terminate`, `clean`, and `attach` are implemented
- `--json` output is implemented
- partial UUID lookup is implemented when the prefix is unique
- localized help text is implemented for English and Simplified Chinese

Still incomplete:

- Windows runtime transport and process management are not implemented yet
- PTY sessions do not provide true `stdout` and `stderr` separation; output is effectively merged by the terminal layer
- there are only unit tests today, not full integration tests

## Why

Normal CLI processes are one-shot. If you run `python`, `bash`, `node`, `psql`, or `ipython`, you lose state as soon as the command exits.

`axec` keeps those sessions alive behind a background daemon so you can come back later:

```bash
axec run --name py python3
axec input --session py --timeout 5 "x = 40 + 2"
axec input --session py --timeout 5 "print(x)"
axec cat --session py
```

## Architecture

`axec` uses a daemon plus CLI client model.

- On Unix, the client talks to the daemon over a local Unix socket
- The daemon starts automatically on first use
- Each session runs behind a PTY so interactive tools behave naturally
- Recent output is kept in memory and full output is appended to log files
- Session metadata is stored on disk under `~/.axec/sessions/<uuid>/`

## Build

Requirements:

- Rust stable

Build locally:

```bash
cargo build
```

Run tests:

```bash
cargo test
```

Run the binary from the repo:

```bash
target/debug/axec --help
```

## Quick Start

Start a persistent shell:

```bash
axec run --name shell bash
```

Send input and stream output for a few seconds:

```bash
axec input --session shell --timeout 3 "echo hello"
```

Read full history:

```bash
axec cat --session shell
```

Attach interactively:

```bash
axec attach --session shell
```

Detach from `attach` with `Ctrl+\`.

List sessions:

```bash
axec list
```

Kill one session:

```bash
axec kill --session shell
```

Kill all running sessions:

```bash
axec kill --all
```

Clean exited session directories:

```bash
axec clean
```

## Command Summary

### `run` / `exec`

Start a command in the background and return its session UUID.

Examples:

```bash
axec run --name repl python3
axec exec --name db psql mydb
axec run --timeout 5 python3 -c "import time; [print(i) or time.sleep(1) for i in range(10)]"
axec run --terminate --timeout 3 sleep 100
```

Notes:

- `run` takes a command plus child-process args
- hyphen-prefixed child args are supported directly, for example `axec run wc -l`

### `input`

Send text to an existing session.

Examples:

```bash
axec input --session repl --timeout 3 "print(42)"
printf 'echo from stdin\n' | axec input --session shell --timeout 3 -
```

Note:

- `input` takes one text payload, not `command + args`, so shell quoting matters

### `cat`

Print recorded output.

Examples:

```bash
axec cat --session repl
axec cat --session repl --follow
axec cat --session repl --stderr
```

Note:

- for PTY-backed sessions, `--stderr` is currently limited because terminal output is merged

### `list`

Show tracked sessions and their status.

### `signal`

Send a Unix signal to a session process group or pid.

Examples:

```bash
axec signal --session repl SIGINT
axec signal --session repl TERM
```

### `kill` / `terminate`

Force-kill a session, or all running sessions.

Examples:

```bash
axec kill --session repl
axec terminate --session repl
axec kill --all
```

### `attach`

Open an interactive terminal attached to a running session.

Examples:

```bash
axec attach --session repl
axec attach --session 1234abcd
```

Notes:

- attach requires an interactive terminal
- detach with `Ctrl+\`
- partial UUID matching works when the prefix is unique

## Session Selection

Anywhere `--session` is accepted, you can pass:

- the full UUID
- the session name
- a unique UUID prefix

If a UUID prefix matches multiple sessions, `axec` returns an ambiguity error.

## JSON Output

All non-attach commands support `--json`.

Examples:

```bash
axec list --json
axec run --json --name train python train.py
axec cat --json --session train
```

## Files and State

By default `axec` stores state under:

- runtime socket/pid files: `$XDG_RUNTIME_DIR/axec/` when available, otherwise `~/.axec/axec/`
- session metadata and logs: `~/.axec/sessions/<uuid>/`

Each session directory contains:

- `meta.json`
- `stdout.log`
- `stderr.log`

## Localization

Help output supports:

- English
- Simplified Chinese

Example:

```bash
LANG=zh_CN.UTF-8 LC_ALL= axec --help
```

## Development

Useful commands:

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## CI

This repo includes GitHub Actions workflows for:

- continuous integration on pushes and pull requests
- tagged release artifact builds for Linux and macOS

See [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and [`.github/workflows/release.yml`](.github/workflows/release.yml).

## License

This project is licensed under the MIT License.

See [`LICENSE`](LICENSE).
