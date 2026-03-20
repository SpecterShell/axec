use std::env;
use std::sync::OnceLock;

static USE_CHINESE: OnceLock<bool> = OnceLock::new();

pub fn init_locale() {
    let locale = detect_locale();
    let use_chinese = locale == "zh-CN";
    let _ = USE_CHINESE.set(use_chinese);
    rust_i18n::set_locale(&locale);
}

pub fn text(key: &str) -> &'static str {
    match (is_chinese(), key) {
        (false, "help.app_about") => {
            "Async command execution CLI for long-running REPL and shell sessions"
        }
        (false, "help.json") => "Emit structured JSON responses",
        (false, "help.list_about") => "List tracked sessions",
        (false, "help.run_about") => {
            "Start a command in the background and return immediately unless streaming is requested"
        }
        (false, "help.cat_about") => {
            "Print session output history and optionally follow live output"
        }
        (false, "help.input_about") => "Send input to a running session",
        (false, "help.signal_about") => "Send an OS signal to a running session",
        (false, "help.kill_about") => "Force-kill a running session",
        (false, "help.all") => "Target all tracked running sessions",
        (false, "help.attach_about") => "Interactively attach to a session",
        (false, "help.clean_about") => "Remove exited sessions and their on-disk state",
        (false, "help.name") => "Optional unique name for the session",
        (false, "help.timeout") => "Stream output for N seconds before returning",
        (false, "help.terminate") => {
            "Terminate the session if the timeout is reached or wait until it exits naturally"
        }
        (false, "help.cwd") => "Working directory for the spawned command",
        (false, "help.env") => "Environment override in K=V form",
        (false, "help.command") => "Command to execute",
        (false, "help.follow") => "Continue streaming live output after printing history",
        (false, "help.stderr") => "Show stderr history instead of stdout history",
        (false, "help.input_text") => "Text to send to the session, or - to read stdin",
        (false, "help.signal") => "Signal name or number",
        (false, "help.session") => "Session UUID or active name",
        (false, "help.missing_subcommand") => "A subcommand is required",
        (false, "help.unknown_command") => "Unknown command",
        (false, "help.invalid_env") => "--env expects K=V",
        (true, "help.app_about") => "用于管理长时间运行 REPL 和命令会话的异步命令执行 CLI",
        (true, "help.json") => "输出结构化 JSON 响应",
        (true, "help.list_about") => "列出当前跟踪的会话",
        (true, "help.run_about") => "启动后台命令；只有请求流式输出时才保持前台等待",
        (true, "help.cat_about") => "打印会话输出历史，并可选择持续跟随新输出",
        (true, "help.input_about") => "向正在运行的会话发送输入",
        (true, "help.signal_about") => "向正在运行的会话发送操作系统信号",
        (true, "help.kill_about") => "强制终止正在运行的会话",
        (true, "help.all") => "作用于所有正在运行且被跟踪的会话",
        (true, "help.attach_about") => "交互式附加到某个会话",
        (true, "help.clean_about") => "删除已退出的会话及其磁盘状态",
        (true, "help.name") => "会话的可选唯一名称",
        (true, "help.timeout") => "流式输出 N 秒后返回",
        (true, "help.terminate") => "达到超时时终止会话，或一直等待到其自然退出",
        (true, "help.cwd") => "启动命令时使用的工作目录",
        (true, "help.env") => "以 K=V 形式设置环境变量覆盖",
        (true, "help.command") => "要执行的命令",
        (true, "help.follow") => "打印历史后继续流式跟随新输出",
        (true, "help.stderr") => "显示 stderr 历史而不是 stdout 历史",
        (true, "help.input_text") => "要发送到会话的文本，或使用 - 从 stdin 读取",
        (true, "help.signal") => "信号名称或编号",
        (true, "help.session") => "会话 UUID 或活动名称",
        (true, "help.missing_subcommand") => "必须提供子命令",
        (true, "help.unknown_command") => "未知命令",
        (true, "help.invalid_env") => "--env 需要 K=V 格式",
        (_, _) => "",
    }
}

fn is_chinese() -> bool {
    *USE_CHINESE.get_or_init(|| detect_locale() == "zh-CN")
}

fn detect_locale() -> String {
    let raw = [
        env::var("LC_ALL").ok(),
        env::var("LANG").ok(),
        env::var("LC_MESSAGES").ok(),
    ]
    .into_iter()
    .flatten()
    .find(|value| !value.is_empty() && !is_neutral_locale(value))
    .unwrap_or_else(|| "en_US".to_string());

    let normalized = raw.replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh") {
        "zh-CN".to_string()
    } else {
        "en".to_string()
    }
}

fn is_neutral_locale(value: &str) -> bool {
    matches!(value, "C" | "POSIX") || value.starts_with("C.")
}
