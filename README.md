# axec

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

[![Build](https://img.shields.io/badge/build-passing-success)](https://github.com/SpecterShell/axec)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**axec** is a daemon-backed CLI designed specifically for agentic coding tools like Claude Code and OpenAI Codex. It lets AI assistants execute code in persistent background sessions, manage long-running tasks, and progressively capture output without the fragility of timeouts or the complexity of MCP servers.

Agentic coding assistants need to:
- Execute Python scripts that take hours to complete
- Run JavaScript/TypeScript code and capture results progressively
- Debug interactively without losing session state

However, traditional approaches are fragile:
- MCP tools take a lot of context, impacting LLM model performance
- CLI tools are stateless and one-shot - not suitable for session-based tasks like browser automation
- Not all libraries provide MCP or CLI tools
- Timeouts require guessing how long code takes to run

axec and axrepl provides durable, programmatic session management designed for agentic workflows.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────────────────────────┐
│  AI Agent   │────▶│    axec     │────▶│  Daemon                             │
│  (Claude/   │◀────│   Client    │◀────│  ┌─────────┐ ┌─────────┐ ┌────────┐ │
│   Codex)    │     │             │     │  │ Python  │ │  Node   │ │ Shell  │ │
└─────────────┘     └─────────────┘     │  │ Session │ │ Session │ │Session │ │
                                        │  └─────────┘ └─────────┘ └────────┘ │
                                        └─────────────────────────────────────┘
```

- use `axec` for detached commands, shells, and long-lived session management
- use `axrepl` when REPL input should return only after the submitted script has finished
- send more input later without losing process state
- stream live output or read historical and incremental logs from later CLI calls
- list, signal, kill, clean, or attach to sessions behind the same daemon

## Two Tools for Agentic Workflows

### `axec` — General Session Management

For running shell commands, servers, and background jobs with full control over signals, output streaming, and stdin/stderr separation.

### `axrepl` — REPL Automation That Just Works

For Python, Node.js, Bash, and Zsh—execute code with **automatic completion detection**. No more guessing timeouts or parsing prompts.

```bash
# Traditional agent approach (fragile)
python -c "print(sum(range(1000)))"  # Hope it finishes in time!

# axrepl approach (reliable)
axrepl run --name py python3
axrepl input --session py "sum(range(1000))"
# 499500
```

## Quick Start

```bash
# 1. Start a persistent Python session for data work
axrepl run --name python python3

# 2. Execute code—automatically waits for completion
axrepl input --session python "
import pandas as pd
df = pd.read_csv('data.csv')
print(df.describe())
"

# 3. Check status anytime
axrepl list

# 4. Clean up when done
axrepl kill --session python
axrepl clean
```

## Examples

### Python Data Analysis Session

```bash
# Start a named Python session
axrepl run --name analysis python3

# Load and explore data
axrepl input --session analysis "
import pandas as pd
import matplotlib.pyplot as plt
df = pd.read_csv('sales.csv')
print(f'Dataset shape: {df.shape}')
print(df.head())
"

# Run analysis (takes time—no timeout needed!)
axrepl input --session analysis "
results = df.groupby('region').revenue.sum()
print(results)
results.to_csv('summary.csv')
"

# Session persists—even if the agent disconnects and reconnects hours later
axrepl input --session analysis "print('Still here!')"
```

### Web Browser Automation

```bash
# Start a Python session for browser automation
axrepl run --name browser python3

# Install and use Playwright
axrepl input --session browser "
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    page.goto('https://example.com')
    title = page.title()
    print(f'Page title: {title}')
"

# The browser session stays open for more interactions
axrepl input --session browser "print('Browser session ready for more tasks')"
```

### Development Server Management

```bash
# Start server in background
axec run --name backend --cwd ./api npm run dev

# Check logs
axec output --session backend

# Still running hours later...
axec list
# 2a40f9d2... backend running npm run dev

# Stop when needed
axec kill --session backend
```

### Complete Agentic Workflow

```bash
# ╔══════════════════════════════════════════════════════════════╗
# ║  AGENT WORKFLOW: Debugging a Python Script                   ║
# ╚══════════════════════════════════════════════════════════════╝

# 1. Create persistent session
axrepl run --name debug python3

# 2. Load the problematic code
axrepl input --session debug "exec(open('script.py').read())"
# [Error traceback shown]

# 3. User disconnects, reconnects hours later
#    Session is still there with all variables intact!

# 4. Debug interactively
axrepl input --session debug "
import pdb
pdb.run('problematic_function()')
"

# Or attach interactively:
axec attach --session debug
# (pdb) commands here
# Press Ctrl+\ to detach

# 5. Fix and verify
axrepl input --session debug "
# Fixed code here
print('Fixed!')
"

# 6. Clean up
axrepl kill --session debug
axrepl clean
```

## Build

```bash
# Clone and build
git clone https://github.com/SpecterShell/axec
cd axec
cargo build --release

# Binaries available at:
# ./target/release/axec
# ./target/release/axrepl

# Optional: Add to PATH for agent access
export PATH="$PATH:$(pwd)/target/release"
```

## Documentation

- **[Complete Guide](docs/guide.md)** — Full walkthrough with agent-focused examples

## License

MIT License — See [LICENSE](LICENSE)
