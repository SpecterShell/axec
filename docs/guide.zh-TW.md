# 使用指南

[English](guide.md) | [简体中文](guide.zh-CN.md) | [繁體中文](guide.zh-TW.md)

此儲存庫提供兩個基於守護程序的 CLI。`axec` 用於通用持久化命令工作階段，`axrepl` 用於具備完成感知能力的 REPL 自動化。

## 功能總覽

`axec` 是通用工作階段管理器，適合獨立 Shell、背景任務，以及透過逾時或 stopword 就足以判斷輸出邊界的 REPL 工作階段。

`axrepl` 是面向 REPL 的分支 CLI。它始終使用 PTY 後端，並在每次提交的腳本外包裹驅動專屬的完成標記，因此 `input` 會在 REPL 實際處理完該腳本後才返回。

目前 `axrepl` 支援 Python、Node、Bash 與 Zsh。守護程序會在首次呼叫時自動啟動，所有工作階段將透過 UUID、可選名稱、狀態、時間戳、啟動指令與日誌檔進行追蹤。

## axec 工作流程

1. 啟動新工作階段前可先檢視當前工作階段狀態：

   ```bash
   axec list
   axec list --json
   ```

2. 如需重複使用工作階段，可啟動具名工作階段：

   ```bash
   axec run --name py python3
   axec run --name shell bash
   ```

3. 後續可向已存在的工作階段傳送輸入：

   ```bash
   axec input --session py --timeout 3 "print(40 + 2)"
   axec input --session shell "echo ready"
   ```

4. 可依場景選擇合適的輸出讀取指令：

   ```bash
   axec output --session py
   axec cat --session py
   axec cat --session py --stderr
   axec cat --session py --follow
   ```

5. 清理測試用途的臨時工作階段：

   ```bash
   axec kill --session py
   axec clean
   ```

## axec 進階用法

- 透過 `-` 參數從標準輸入傳送多行內容：

  ```bash
  axec input --session py - <<'EOF'
  x = 40
  y = 2
  print(x + y)
  EOF
  ```

- 透過管線傳遞多行輸入：

  ```bash
  printf 'first line\nsecond line\n' | axec input --session shell -
  ```

- 傳送輸入後等待指定終止詞（stopword）回傳：

  ```bash
  axec input --session py --stopword 'ready|done' "run_job()"
  ```

- 啟動非互動式命令並保持標準輸出與標準錯誤分離：

  ```bash
  axec run --name build --backend pipe sh -c 'echo out; echo err >&2'
  axec cat --session build
  axec cat --session build --stderr
  ```

- 執行一次性檢查類命令時使用 `--backend auto` 參數：

  ```bash
  axec run --name check --backend auto sh -c 'echo ok; echo warn >&2; sleep 1'
  axec list --json
  ```

- 直接重複使用最近建立的工作階段：

  ```bash
  axec output
  axec input "echo follow-up"
  ```

- 透過 UUID 唯一前綴定位工作階段：

  ```bash
  axec list
  axec cat --session 2a40f9d2
  ```

- 輸出歷史日誌後持續追蹤即時輸出：

  ```bash
  axec cat --session build --follow
  ```

- 強制終止工作階段前先傳送優雅中斷訊號：

  ```bash
  axec signal --session py SIGINT
  axec kill --session py
  ```

## axrepl 工作流程

1. 使用受支援的驅動啟動具名 REPL 工作階段：

   ```bash
   axrepl run --name py python3
   axrepl run --name js node
   ```

2. 傳送腳本，並讓 REPL 自行判定何時處理完成：

   ```bash
   axrepl input --session py "print(40 + 2)"
   axrepl input --session js "console.log(40 + 2)"
   ```

3. 透過標準輸入傳送多行腳本，不需手動撰寫 stopword：

   ```bash
   axrepl input --session py - <<'EOF'
   x = 40
   y = 2
   print(x + y)
   EOF
   ```

4. 透過分支 CLI 管理 REPL 專用工作階段：

   ```bash
   axrepl list
   axrepl kill --session py
   axrepl clean
   ```

## axec 指令參考

| 指令 | 功能說明 |
|---|---|
| `axec run [--name NAME] [--timeout N] [--terminate] [--stopword REGEX] [--backend pty\|pipe\|auto] [--cwd DIR] [--env K=V]... <cmd> [args]` | 啟動新工作階段，立即回傳工作階段 UUID，可搭配 `--timeout` 或 `--stopword` 串流讀取初始輸出。 |
| `axec run --backend pipe <cmd>` | `--backend pty` 為預設值，會合併終端的標準輸出與標準錯誤；`pipe` 模式會強制分離兩個輸出串流；`auto` 模式會透過啟發式策略，在非互動場景下優先選擇 `pipe`。 |
| `axec cat [--session UUID\|NAME] [--follow] [--stderr]` | 預設輸出已記錄的標準輸出，加入 `--stderr` 參數時輸出標準錯誤，可搭配 `--follow` 即時追蹤後續輸出。 |
| `axec output [--session UUID\|NAME]` | 輸出該工作階段自上次輸出類指令執行後新增的標準輸出內容，省略 `--session` 時預設使用最近建立的工作階段。 |
| `axec list` | 顯示所有已追蹤工作階段的 UUID、名稱、狀態、啟動時間、結束時間與啟動指令。 |
| `axec input [--session UUID\|NAME] [--timeout N] [--stopword REGEX] [--terminate] <text>` | 向執行中的工作階段傳送文字內容，可選擇開啟串流輸出讀取，可設定正規比對終止條件。 |
| `axec signal [--session UUID\|NAME] <SIGNAL>` | 向工作階段傳送作業系統訊號（如 `SIGINT`），省略 `--session` 時預設使用最近建立的工作階段。 |
| `axec kill --session UUID\|NAME` / `axec kill --all` | 強制終止指定工作階段，搭配 `--all` 參數可終止所有執行中的工作階段。 |
| `axec clean` / `axec clear` | 刪除已退出的工作階段及其磁碟儲存的狀態資料，`clear` 為該指令的別名。 |
| `axec attach --session UUID\|NAME` | 開啟互動式終端連接執行中的工作階段，使用 `Ctrl+\` 快速鍵中斷工作階段連線。 |

## axrepl 指令參考

| 指令 | 功能說明 |
|---|---|
| `axrepl run [--name NAME] [--driver python\|node\|bash\|zsh] [--cwd DIR] [--env K=V]... <cmd> [args]` | 啟動基於 PTY 的 REPL 工作階段，並記錄已辨識的驅動，供後續完成感知輸入使用。 |
| `axrepl input [--session UUID\|NAME] [--driver python\|node\|bash\|zsh] <text\|->` | 包裹提交的腳本、等待 REPL 回傳完成標記，並輸出清理後的結果。省略 `--session` 時預設使用最近的 REPL 工作階段。 |
| `axrepl list` | 顯示所有已辨識出 REPL 驅動的已追蹤工作階段。 |
| `axrepl kill --session UUID\|NAME` / `axrepl kill --all` | 強制終止指定 REPL 工作階段，搭配 `--all` 參數可終止所有執行中的 REPL 工作階段。 |
| `axrepl clean` | 刪除已退出的 REPL 工作階段及其磁碟儲存狀態。 |

## 輸出模式說明

`pty` 是 `axec` 的預設後端，也是 `axrepl` 唯一使用的後端，適用於全互動終端場景，並會合併標準輸出與標準錯誤。

`pipe` 適用於一次性命令與結構化工具場景，會分離兩個輸出串流；`auto` 模式會基於平台特性自動判斷，非互動負載下通常會優先選擇 `pipe`。

## 工作階段選擇規則

所有支援 `--session` 參數的指令，均可傳入完整 UUID、唯一 UUID 前綴或活躍工作階段的名稱定位目標工作階段。

`cat`、`output`、`input`、`signal` 四個指令支援省略 `--session` 參數，此時會自動選擇最近啟動的工作階段。

`axrepl input` 也支援省略 `--session`，此時會自動選擇最近的 REPL 工作階段。

## 多語言

兩個 CLI 的說明資訊都會跟隨系統區域設定環境變數展示對應語言。如需固定語言，可使用 `AXEC_LOCALE` 變數覆蓋 `LANG` 與 `LC_*` 系列變數，範例：

```bash
AXEC_LOCALE=zh-TW axec --help
AXEC_LOCALE=zh-TW axrepl --help
LANG=zh_CN.UTF-8 axec --help
LANG=zh_TW.UTF-8 axec --help
```

## 儲存路徑與狀態

預設情況下，執行階段 socket 與 pid 檔案會存放在 `$XDG_RUNTIME_DIR/axec/` 目錄下；若該路徑不可用，則會改用 `~/.axec/axec/`。

工作階段中繼資料與日誌會存放在 `~/.axec/sessions/<uuid>/` 目錄下，包含 `meta.json`、`stdout.log` 與 `stderr.log`。

`axrepl` 還會在對應的 REPL 工作階段目錄中寫入 `axrepl.json`，供後續 `input` 呼叫重用已辨識的驅動資訊。
