# axec

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

[![Build](https://img.shields.io/badge/build-passing-success)](https://github.com/SpecterShell/axec)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**axec** 是一個專為 Claude Code 和 OpenAI Codex 等代理式編程工具設計、由守護程序支援的 CLI。它讓 AI 助手能夠在持久的背景工作階段中執行程式碼、管理長時間執行的任務，並持續擷取輸出，而不必依賴脆弱的逾時機制，也不需要承擔 MCP 伺服器帶來的複雜性。

代理式編程助手需要：
- 執行可能需要數小時才能完成的 Python 指令碼
- 執行 JavaScript/TypeScript 程式碼並逐步擷取結果
- 在不遺失工作階段狀態的前提下進行互動式偵錯

但傳統方式很脆弱：
- MCP 工具會佔用大量上下文，影響 LLM 的表現
- CLI 工具是無狀態、一次性的，不適合瀏覽器自動化這類基於工作階段的任務
- 並不是所有函式庫都提供 MCP 或 CLI 工具
- 逾時機制要求你預估程式碼到底要執行多久

axec 和 axrepl 提供了面向代理式工作流的持久化、可程式化工作階段管理。

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────────────────────────┐
│   AI 代理    │────▶│    axec     │────▶│  守護程序                           │
│  (Claude/   │◀────│    用戶端    │◀────│  ┌─────────┐ ┌─────────┐ ┌────────┐ │
│   Codex)    │     │             │     │  │ Python  │ │  Node   │ │ Shell  │ │
└─────────────┘     └─────────────┘     │  │ 工作階段 │ │ 工作階段 │ │工作階段│ │
                                        │  └─────────┘ └─────────┘ └────────┘ │
                                        └─────────────────────────────────────┘
```

- 使用 `axec` 處理脫離終端的命令、shell 與長生命週期工作階段管理
- 使用 `axrepl`，讓提交的腳本在真正執行完成後才返回結果
- 後續可繼續向同一工作階段傳送輸入，而不會遺失程序狀態
- 串流檢視即時輸出，或在之後的 CLI 呼叫中讀取歷史與增量日誌
- 透過同一個守護程序列出、發送信號、終止、清理或接入工作階段

## 面向代理式工作流的兩個工具

### `axec` - 通用工作階段管理

用於執行 shell 命令、伺服器與背景任務，並完整控制信號、輸出串流，以及 stdin/stderr 分離。

### `axrepl` - 開箱即用的 REPL 自動化

適用於 Python、Node.js、Bash 和 Zsh，可在執行程式碼時進行**自動完成偵測**。不再需要猜測逾時，也不必解析提示字元。

```bash
# 傳統代理方案（脆弱）
python -c "print(sum(range(1000)))"  # 只能賭它會及時跑完！

# axrepl 方案（可靠）
axrepl run --name py python3
axrepl input --session py "sum(range(1000))"
# 499500
```

## 快速開始

```bash
# 1. 啟動一個用於資料處理的持久 Python 工作階段
axrepl run --name python python3

# 2. 執行程式碼，自動等待完成
axrepl input --session python "
import pandas as pd
df = pd.read_csv('data.csv')
print(df.describe())
"

# 3. 隨時查看狀態
axrepl list

# 4. 完成後清理
axrepl kill --session python
axrepl clean
```

## 範例

### Python 資料分析工作階段

```bash
# 啟動一個具名 Python 工作階段
axrepl run --name analysis python3

# 載入並探索資料
axrepl input --session analysis "
import pandas as pd
import matplotlib.pyplot as plt
df = pd.read_csv('sales.csv')
print(f'資料集形狀: {df.shape}')
print(df.head())
"

# 執行分析（耗時較長，也不需要逾時）
axrepl input --session analysis "
results = df.groupby('region').revenue.sum()
print(results)
results.to_csv('summary.csv')
"

# 即使代理斷開連線，數小時後回來工作階段仍然存在
axrepl input --session analysis "print('還在這裡！')"
```

### Web 瀏覽器自動化

```bash
# 為瀏覽器自動化啟動一個 Python 工作階段
axrepl run --name browser python3

# 安裝並使用 Playwright
axrepl input --session browser "
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    page.goto('https://example.com')
    title = page.title()
    print(f'頁面標題: {title}')
"

# 瀏覽器工作階段保持開啟，可繼續執行更多互動
axrepl input --session browser "print('瀏覽器工作階段已準備好執行更多任務')"
```

### 開發伺服器管理

```bash
# 在背景啟動伺服器
axec run --name backend --cwd ./api npm run dev

# 查看日誌
axec output --session backend

# 幾小時後它仍然在執行...
axec list
# 2a40f9d2... backend running npm run dev

# 需要時停止
axec kill --session backend
```

### 完整的代理式工作流

```bash
# ╔══════════════════════════════════════════════════════════════╗
# ║  AGENT WORKFLOW: 偵錯一個 Python 指令碼                      ║
# ╚══════════════════════════════════════════════════════════════╝

# 1. 建立持久工作階段
axrepl run --name debug python3

# 2. 載入有問題的程式碼
axrepl input --session debug "exec(open('script.py').read())"
# [顯示錯誤回溯]

# 3. 使用者斷開連線，幾小時後重新連上
#    工作階段仍然存在，所有變數都還在

# 4. 互動式偵錯
axrepl input --session debug "
import pdb
pdb.run('problematic_function()')
"

# 或者直接接入互動工作階段：
axec attach --session debug
# (pdb) 在這裡輸入命令
# 按 Ctrl+\ 脫離

# 5. 修復並驗證
axrepl input --session debug "
# 在這裡寫入修復後的程式碼
print('已修復！')
"

# 6. 清理
axrepl kill --session debug
axrepl clean
```

## 構建

```bash
# 複製並構建
git clone https://github.com/SpecterShell/axec
cd axec
cargo build --release

# 二進位檔位於：
# ./target/release/axec
# ./target/release/axrepl

# 可選：加入 PATH，方便代理呼叫
export PATH="$PATH:$(pwd)/target/release"
```

## 文件

- **[完整指南](docs/guide.zh-TW.md)** - 包含面向代理示例的完整導覽

## 授權條款

MIT 授權條款 - 參見 [LICENSE](LICENSE)
