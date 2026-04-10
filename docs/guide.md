# Guide

[English](guide.md) | [简体中文](guide.zh-CN.md) | [繁體中文](guide.zh-TW.md)

This repo ships two daemon-backed CLIs. Use `axec` for general persistent command sessions, and use `axrepl` for completion-aware REPL automation.

## Overview

`axec` is the general session manager. It works well for detached shells, background jobs, and REPL sessions where timeout-based or stopword-based reads are good enough.

`axrepl` is a REPL-focused fork. It always uses the PTY backend and wraps each submitted script with a driver-specific completion marker so `input` can return after the REPL has actually finished processing that script.

Supported `axrepl` drivers today are Python, Node, Bash, and Zsh. The daemon starts automatically on first use, and sessions are tracked by UUID, optional name, status, timestamps, command line, and log files.

## axec Workflow

1. Inspect current state before starting new sessions:

   ```bash
   axec list
   axec list --json
   ```

2. Start a named session when the session will be reused:

   ```bash
   axec run --name py python3
   axec run --name shell bash
   ```

3. Send more input later:

   ```bash
   axec input --session py --timeout 3 "print(40 + 2)"
   axec input --session shell "echo ready"
   ```

4. Read output with the right command:

   ```bash
   axec output --session py
   axec cat --session py
   axec cat --session py --stderr
   axec cat --session py --follow
   ```

5. Clean up sessions started for verification:

   ```bash
   axec kill --session py
   axec clean
   ```

## axec Recipes

- Send multi-line input through stdin with `-`:

  ```bash
  axec input --session py - <<'EOF'
  x = 40
  y = 2
  print(x + y)
  EOF
  ```

- Send multi-line input from a pipe:

  ```bash
  printf 'first line\nsecond line\n' | axec input --session shell -
  ```

- Wait for a stopword while sending input:

  ```bash
  axec input --session py --stopword 'ready|done' "run_job()"
  ```

- Start a non-interactive command and keep stdout and stderr separated:

  ```bash
  axec run --name build --backend pipe sh -c 'echo out; echo err >&2'
  axec cat --session build
  axec cat --session build --stderr
  ```

- Use `--backend auto` for one-shot command checks:

  ```bash
  axec run --name check --backend auto sh -c 'echo ok; echo warn >&2; sleep 1'
  axec list --json
  ```

- Reuse the latest session intentionally:

  ```bash
  axec output
  axec input "echo follow-up"
  ```

- Target a session by unique UUID prefix:

  ```bash
  axec list
  axec cat --session 2a40f9d2
  ```

- Follow live output after printing history:

  ```bash
  axec cat --session build --follow
  ```

- Interrupt gracefully before force-killing:

  ```bash
  axec signal --session py SIGINT
  axec kill --session py
  ```

## axrepl Workflow

1. Start a named REPL session with a supported driver:

   ```bash
   axrepl run --name py python3
   axrepl run --name js node
   ```

2. Send a script and let the REPL decide when it is done:

   ```bash
   axrepl input --session py "print(40 + 2)"
   axrepl input --session js "console.log(40 + 2)"
   ```

3. Send multi-line scripts over stdin without hand-written stopwords:

   ```bash
   axrepl input --session py - <<'EOF'
   x = 40
   y = 2
   print(x + y)
   EOF
   ```

4. Manage REPL-only sessions through the forked CLI:

   ```bash
   axrepl list
   axrepl kill --session py
   axrepl clean
   ```

## axec Reference

| Command | Behavior |
|---|---|
| `axec run [--name NAME] [--timeout N] [--terminate] [--stopword REGEX] [--backend pty\|pipe\|auto] [--cwd DIR] [--env K=V]... <cmd> [args]` | Start a session, return its UUID immediately, and optionally stream early output with `--timeout` or `--stopword`. |
| `axec run --backend pipe <cmd>` | `--backend pty` is the default and keeps merged terminal output; `pipe` forces split stdout/stderr, while `auto` uses heuristics to prefer `pipe` for non-interactive commands. |
| `axec cat [--session UUID\|NAME] [--follow] [--stderr]` | Print recorded stdout by default, `--stderr` when requested, and follow live output with `--follow`. |
| `axec output [--session UUID\|NAME]` | Print unread stdout since the last output-aware command for that session. If `--session` is omitted, the latest session is used. |
| `axec list` | Show tracked sessions with UUID, name, status, start time, exit time, and command line. |
| `axec input [--session UUID\|NAME] [--timeout N] [--stopword REGEX] [--terminate] <text>` | Send text to a running session, optionally stream output, and optionally stop once a regex matches. |
| `axec signal [--session UUID\|NAME] <SIGNAL>` | Send an OS signal such as `SIGINT`; if `--session` is omitted, the latest session is used. |
| `axec kill --session UUID\|NAME` / `axec kill --all` | Force-kill a specific session or all running sessions with `--all`. |
| `axec clean` / `axec clear` | Remove exited sessions and their on-disk state. `clear` is an alias. |
| `axec attach --session UUID\|NAME` | Open an interactive terminal attached to a running session. Detach with `Ctrl+\`. |

## axrepl Reference

| Command | Behavior |
|---|---|
| `axrepl run [--name NAME] [--driver python\|node\|bash\|zsh] [--cwd DIR] [--env K=V]... <cmd> [args]` | Start a PTY-backed REPL session and record the detected driver for later completion-aware input. |
| `axrepl input [--session UUID\|NAME] [--driver python\|node\|bash\|zsh] <text\|->` | Wrap the submitted script, wait for the REPL completion marker, and print cleaned output. If `--session` is omitted, the latest REPL-capable session is used. |
| `axrepl list` | Show tracked sessions that have a known REPL driver. |
| `axrepl kill --session UUID\|NAME` / `axrepl kill --all` | Force-kill a specific REPL session or all running REPL sessions with `--all`. |
| `axrepl clean` | Remove exited REPL sessions and their on-disk state. |

## Output Modes

`pty` is the default backend in `axec`, and the only backend used by `axrepl`. It keeps stdout/stderr merged for fully interactive terminal sessions.

`pipe` keeps them separate for one-shot commands and structured tooling, while `auto` applies platform heuristics and may prefer `pipe` for non-interactive workloads.

## Session Selection

Anywhere `--session` is accepted, you can pass a full UUID, a unique UUID prefix, or the active session name.

`cat`, `output`, `input`, and `signal` also accept an omitted `--session`, in which case the latest started session is used.

`axrepl input` also accepts an omitted `--session`, in which case the latest REPL-capable session is used.

## Localization

The CLI help follows your locale environment. `AXEC_LOCALE` overrides `LANG` and `LC_*` when you need a deterministic locale for either CLI. Examples:

```bash
AXEC_LOCALE=zh-TW axec --help
AXEC_LOCALE=zh-TW axrepl --help
LANG=zh_CN.UTF-8 axec --help
LANG=zh_TW.UTF-8 axec --help
```

## Files and State

By default, runtime socket and pid files live under `$XDG_RUNTIME_DIR/axec/` when available, otherwise `~/.axec/axec/`.

Session metadata and logs live under `~/.axec/sessions/<uuid>/`, including `meta.json`, `stdout.log`, and `stderr.log`.

`axrepl` also writes `axrepl.json` in each REPL session directory so later `input` calls can reuse the detected driver.
