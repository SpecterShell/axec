# 使用指南

[English](guide.md) | [简体中文](guide.zh-CN.md) | [繁體中文](guide.zh-TW.md)

此仓库提供两个基于守护进程的 CLI。`axec` 用于通用持久化命令会话，`axrepl` 用于具备完成感知能力的 REPL 自动化。

## 功能概览

`axec` 是通用会话管理器，适合独立 Shell、后台任务，以及通过超时或 stopword 就足够判断输出边界的 REPL 会话。

`axrepl` 是面向 REPL 的分支 CLI。它始终使用 PTY 后端，并在每次提交的脚本外包裹驱动专属的完成标记，因此 `input` 会在 REPL 实际处理完该脚本后再返回。

当前 `axrepl` 支持 Python、Node、Bash 与 Zsh。守护进程会在首次调用时自动启动，所有会话将通过 UUID、可选名称、状态、时间戳、启动命令与日志文件进行追踪。

## axec 工作流

1. 启动新会话前可先查看当前会话状态：

   ```bash
   axec list
   axec list --json
   ```

2. 如需重复使用会话，可启动命名会话：

   ```bash
   axec run --name py python3
   axec run --name shell bash
   ```

3. 后续可向已存在的会话发送输入：

   ```bash
   axec input --session py --timeout 3 "print(40 + 2)"
   axec input --session shell "echo ready"
   ```

4. 可根据场景选择合适的输出读取命令：

   ```bash
   axec output --session py
   axec cat --session py
   axec cat --session py --stderr
   axec cat --session py --follow
   ```

5. 清理测试使用的临时会话：

   ```bash
   axec kill --session py
   axec clean
   ```

## axec 进阶用法

- 通过 `-` 参数从标准输入发送多行内容：

  ```bash
  axec input --session py - <<'EOF'
  x = 40
  y = 2
  print(x + y)
  EOF
  ```

- 通过管道传递多行输入：

  ```bash
  printf 'first line\nsecond line\n' | axec input --session shell -
  ```

- 发送输入后等待指定终止词（stopword）返回：

  ```bash
  axec input --session py --stopword 'ready|done' "run_job()"
  ```

- 启动非交互式命令并保持标准输出与标准错误独立：

  ```bash
  axec run --name build --backend pipe sh -c 'echo out; echo err >&2'
  axec cat --session build
  axec cat --session build --stderr
  ```

- 执行一次性检查类命令时使用 `--backend auto` 参数：

  ```bash
  axec run --name check --backend auto sh -c 'echo ok; echo warn >&2; sleep 1'
  axec list --json
  ```

- 直接复用最近创建的会话：

  ```bash
  axec output
  axec input "echo follow-up"
  ```

- 通过 UUID 唯一前缀定位会话：

  ```bash
  axec list
  axec cat --session 2a40f9d2
  ```

- 输出历史日志后持续追踪实时输出：

  ```bash
  axec cat --session build --follow
  ```

- 强制终止会话前先发送优雅中断信号：

  ```bash
  axec signal --session py SIGINT
  axec kill --session py
  ```

## axrepl 工作流

1. 使用受支持的驱动启动具名 REPL 会话：

   ```bash
   axrepl run --name py python3
   axrepl run --name js node
   ```

2. 发送脚本，并让 REPL 自行决定何时处理完成：

   ```bash
   axrepl input --session py "print(40 + 2)"
   axrepl input --session js "console.log(40 + 2)"
   ```

3. 通过标准输入发送多行脚本，无需手动编写 stopword：

   ```bash
   axrepl input --session py - <<'EOF'
   x = 40
   y = 2
   print(x + y)
   EOF
   ```

4. 通过分支 CLI 管理 REPL 会话：

   ```bash
   axrepl list
   axrepl kill --session py
   axrepl clean
   ```

## axec 命令参考

| 命令 | 功能 |
|---|---|
| `axec run [--name NAME] [--timeout N] [--terminate] [--stopword REGEX] [--backend pty\|pipe\|auto] [--cwd DIR] [--env K=V]... <cmd> [args]` | 启动新会话，立即返回会话 UUID，可配合 `--timeout` 或 `--stopword` 流式读取初始输出。 |
| `axec run --backend pipe <cmd>` | `--backend pty` 为默认值，会合并输出终端的标准输出与标准错误；`pipe` 模式会强制分离两个输出流；`auto` 模式会通过启发式策略，在非交互场景下优先选择 `pipe`。 |
| `axec cat [--session UUID\|NAME] [--follow] [--stderr]` | 默认输出已记录的标准输出，添加 `--stderr` 参数时输出标准错误，可配合 `--follow` 实时追踪后续输出。 |
| `axec output [--session UUID\|NAME]` | 输出该会话自上次输出类命令执行后新增的标准输出内容，省略 `--session` 时默认使用最近创建的会话。 |
| `axec list` | 展示所有已追踪会话的 UUID、名称、状态、启动时间、退出时间与启动命令。 |
| `axec input [--session UUID\|NAME] [--timeout N] [--stopword REGEX] [--terminate] <text>` | 向运行中的会话发送文本内容，可选开启流式输出读取，可设置正则匹配终止条件。 |
| `axec signal [--session UUID\|NAME] <SIGNAL>` | 向会话发送操作系统信号（如 `SIGINT`），省略 `--session` 时默认使用最近创建的会话。 |
| `axec kill --session UUID\|NAME` / `axec kill --all` | 强制终止指定会话，配合 `--all` 参数可终止所有运行中的会话。 |
| `axec clean` / `axec clear` | 删除已退出的会话及其磁盘存储的状态数据，`clear` 为该命令的别名。 |
| `axec attach --session UUID\|NAME` | 打开交互式终端接入运行中的会话，使用 `Ctrl+\` 快捷键断开会话连接。 |

## axrepl 命令参考

| 命令 | 功能 |
|---|---|
| `axrepl run [--name NAME] [--driver python\|node\|bash\|zsh] [--cwd DIR] [--env K=V]... <cmd> [args]` | 启动基于 PTY 的 REPL 会话，并记录已识别的驱动，供后续完成感知输入使用。 |
| `axrepl input [--session UUID\|NAME] [--driver python\|node\|bash\|zsh] <text\|->` | 包裹提交的脚本、等待 REPL 返回完成标记，并输出清洗后的结果。省略 `--session` 时默认使用最近的 REPL 会话。 |
| `axrepl list` | 展示所有已识别出 REPL 驱动的已追踪会话。 |
| `axrepl kill --session UUID\|NAME` / `axrepl kill --all` | 强制终止指定 REPL 会话，配合 `--all` 参数可终止所有运行中的 REPL 会话。 |
| `axrepl clean` | 删除已退出的 REPL 会话及其磁盘存储状态。 |

## 输出模式

`pty` 是 `axec` 的默认后端，也是 `axrepl` 唯一使用的后端，适用于全交互终端场景，并会合并标准输出与标准错误。

`pipe` 适用于一次性命令与结构化工具场景，会分离两个输出流；`auto` 模式会基于平台特性自动判断，非交互负载下通常会优先选择 `pipe`。

## 会话选择

所有支持 `--session` 参数的命令，均可传入完整 UUID、唯一 UUID 前缀或活跃会话的名称定位目标会话。

`cat`、`output`、`input`、`signal` 四个命令支持省略 `--session` 参数，此时会自动选择最近启动的会话。

`axrepl input` 也支持省略 `--session`，此时会自动选择最近的 REPL 会话。

## 多语言

两个 CLI 的帮助信息都会跟随系统区域设置环境变量展示对应语言。如需固定语言，可使用 `AXEC_LOCALE` 变量覆盖 `LANG` 与 `LC_*` 系列变量，示例：

```bash
AXEC_LOCALE=zh-TW axec --help
AXEC_LOCALE=zh-TW axrepl --help
LANG=zh_CN.UTF-8 axec --help
LANG=zh_TW.UTF-8 axec --help
```

## 存储路径与状态

默认情况下，运行时 socket 与 pid 文件会存储在 `$XDG_RUNTIME_DIR/axec/` 目录下；若该路径不可用，则会使用 `~/.axec/axec/`。

会话元数据与日志会存储在 `~/.axec/sessions/<uuid>/` 目录下，包含 `meta.json`（元数据）、`stdout.log`（标准输出日志）与 `stderr.log`（标准错误日志）。

`axrepl` 还会在对应的 REPL 会话目录中写入 `axrepl.json`，供后续 `input` 调用复用已识别的驱动信息。
