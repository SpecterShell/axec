# axec

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

[![Build](https://img.shields.io/badge/build-passing-success)](https://github.com/SpecterShell/axec)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**axec** 是一个专为 Claude Code 和 OpenAI Codex 等智能体编程工具设计、由守护进程支持的 CLI。它让 AI 助手能够在持久的后台会话中执行代码、管理长时间运行的任务，并持续捕获输出，而不必依赖脆弱的超时机制，也不需要承担 MCP 服务器带来的复杂性。

智能体编程助手需要：
- 执行可能要运行数小时的 Python 脚本
- 运行 JavaScript/TypeScript 代码并逐步捕获结果
- 在不丢失会话状态的前提下进行交互式调试

但传统方式很脆弱：
- MCP 工具会占用大量上下文，影响 LLM 的表现
- CLI 工具是无状态、一次性的，不适合浏览器自动化这类基于会话的任务
- 并不是所有库都提供 MCP 或 CLI 工具
- 超时机制要求你预估代码到底要运行多久

axec 和 axrepl 提供了面向智能体工作流的持久化、可编程会话管理。

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────────────────────────┐
│   AI 智能体  │────▶│    axec     │────▶│  守护进程                           │
│  (Claude/   │◀────│    客户端    │◀────│  ┌─────────┐ ┌─────────┐ ┌────────┐ │
│   Codex)    │     │             │     │  │ Python  │ │  Node   │ │ Shell  │ │
└─────────────┘     └─────────────┘     │  │  会话    │ │  会话    │ │ 会话   │ │
                                        │  └─────────┘ └─────────┘ └────────┘ │
                                        └─────────────────────────────────────┘
```

- 使用 `axec` 处理脱离终端的命令、shell 和长生命周期会话管理
- 使用 `axrepl` 在提交的脚本真正执行完成后再返回结果
- 后续继续向同一会话发送输入，而不会丢失进程状态
- 流式查看实时输出，或在之后的 CLI 调用中读取历史与增量日志
- 通过同一个守护进程列出、发信号、终止、清理或接入会话

## 面向智能体工作流的两个工具

### `axec` - 通用会话管理

用于运行 shell 命令、服务器和后台任务，并完整控制信号、输出流，以及 stdin/stderr 分离。

### `axrepl` - 开箱即用的 REPL 自动化

适用于 Python、Node.js、Bash 和 Zsh，可在执行代码时进行**自动完成检测**。不再需要猜测超时，也不必解析提示符。

```bash
# 传统智能体方案（脆弱）
python -c "print(sum(range(1000)))"  # 只能赌它会及时跑完！

# axrepl 方案（可靠）
axrepl run --name py python3
axrepl input --session py "sum(range(1000))"
# 499500
```

## 快速开始

```bash
# 1. 启动一个用于数据处理的持久 Python 会话
axrepl run --name python python3

# 2. 执行代码，自动等待完成
axrepl input --session python "
import pandas as pd
df = pd.read_csv('data.csv')
print(df.describe())
"

# 3. 随时查看状态
axrepl list

# 4. 完成后清理
axrepl kill --session python
axrepl clean
```

## 示例

### Python 数据分析会话

```bash
# 启动一个具名 Python 会话
axrepl run --name analysis python3

# 加载并探索数据
axrepl input --session analysis "
import pandas as pd
import matplotlib.pyplot as plt
df = pd.read_csv('sales.csv')
print(f'数据集形状: {df.shape}')
print(df.head())
"

# 运行分析（耗时较长，也不需要超时）
axrepl input --session analysis "
results = df.groupby('region').revenue.sum()
print(results)
results.to_csv('summary.csv')
"

# 即使智能体断开连接，数小时后回来会话仍然存在
axrepl input --session analysis "print('还在这里！')"
```

### Web 浏览器自动化

```bash
# 为浏览器自动化启动一个 Python 会话
axrepl run --name browser python3

# 安装并使用 Playwright
axrepl input --session browser "
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    page.goto('https://example.com')
    title = page.title()
    print(f'页面标题: {title}')
"

# 浏览器会话保持开启，可继续执行更多交互
axrepl input --session browser "print('浏览器会话已准备好执行更多任务')"
```

### 开发服务器管理

```bash
# 在后台启动服务器
axec run --name backend --cwd ./api npm run dev

# 查看日志
axec output --session backend

# 几小时后它仍然在运行...
axec list
# 2a40f9d2... backend running npm run dev

# 需要时停止
axec kill --session backend
```

### 完整的智能体工作流

```bash
# ╔══════════════════════════════════════════════════════════════╗
# ║  AGENT WORKFLOW: 调试一个 Python 脚本                        ║
# ╚══════════════════════════════════════════════════════════════╝

# 1. 创建持久会话
axrepl run --name debug python3

# 2. 加载有问题的代码
axrepl input --session debug "exec(open('script.py').read())"
# [显示错误回溯]

# 3. 用户断开连接，几小时后重新连上
#    会话仍然存在，所有变量都还在

# 4. 交互式调试
axrepl input --session debug "
import pdb
pdb.run('problematic_function()')
"

# 或者直接接入交互会话：
axec attach --session debug
# (pdb) 在这里输入命令
# 按 Ctrl+\ 脱离

# 5. 修复并验证
axrepl input --session debug "
# 在这里写入修复后的代码
print('已修复！')
"

# 6. 清理
axrepl kill --session debug
axrepl clean
```

## 构建

```bash
# 克隆并构建
git clone https://github.com/SpecterShell/axec
cd axec
cargo build --release

# 二进制文件位于：
# ./target/release/axec
# ./target/release/axrepl

# 可选：加入 PATH，方便智能体调用
export PATH="$PATH:$(pwd)/target/release"
```

## 文档

- **[完整指南](docs/guide.zh-CN.md)** - 包含面向智能体示例的完整演练

## 许可证

MIT 许可证 - 参见 [LICENSE](LICENSE)
