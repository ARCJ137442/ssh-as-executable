use std::env;
use std::io::{self, Read};
use std::process::{Command, Stdio};

#[allow(dead_code)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

const ASKPASS_MODE_ENV: &str = "SSH_AS_EXECUTABLE_ASKPASS";
const ASKPASS_TOKEN_ENV: &str = "SSH_AS_EXECUTABLE_ASKPASS_TOKEN";

fn main() {
    // 密码模式的关键是让本 exe 具备“双身份”：
    // 1. 用户直接运行时，它负责拼出 ssh 命令并启动 OpenSSH。
    // 2. OpenSSH 需要密码时，会按 SSH_ASKPASS 再启动本 exe；这时只输出封装密码。
    // 这不是绕过 SSH 认证，而是把“人手输入密码”替换成 askpass helper 自动响应。
    if env::var_os(ASKPASS_MODE_ENV).is_some() {
        // token 只用于区分“被本程序启动的 askpass 调用”和手工误触发；
        // 密码仍然是 exe 内部可恢复的数据，不等同于系统凭据库或硬件密钥保护。
        if env::var(ASKPASS_TOKEN_ENV).unwrap_or_default() == generated::get_askpass_token() {
            println!("{}", generated::get_password());
            return;
        }
        std::process::exit(1);
    }

    let args: Vec<String> = env::args().collect();

    // --help 显示帮助
    if args.len() > 1 && args[1] == "--help" {
        println!("{}", generated::get_help());
        return;
    }

    let host = generated::get_host();
    let user = generated::get_user();
    let auth_mode = generated::get_auth_mode();
    let ssh_cmd = generated::get_ssh_cmd();
    let ssh_flag = generated::get_ssh_flag();
    let port = generated::get_port();

    // 解析命令行参数
    let (target, cmd_to_run) = if args.len() > 1 && args[1] == "--stdin" {
        // --stdin：从 stdin 读取命令
        let mut stdin_input = String::new();
        if io::stdin().read_to_string(&mut stdin_input).is_ok() {
            (format!("{}@{}", user, host), stdin_input.trim().to_string())
        } else {
            (format!("{}@{}", user, host), String::new())
        }
    } else if args.len() > 1 {
        // 命令行有参数
        let a = &args[1];
        if a.contains('@') {
            (a.clone(), args[2..].join(" "))
        } else {
            (format!("{}@{}", user, host), args[1..].join(" "))
        }
    } else {
        // 无参数：交互式 SSH
        (format!("{}@{}", user, host), String::new())
    };

    let password_mode = auth_mode.eq_ignore_ascii_case("password");
    let interactive = cmd_to_run.is_empty();
    let key_path = if password_mode {
        String::new()
    } else {
        generated::get_key_path()
    };

    let mut ssh_args: Vec<String> = vec!["-o".to_string(), ssh_flag];

    if password_mode {
        // 让 password 模式尽量不受调用机器上已有 SSH agent / 默认私钥影响：
        // 明确优先 password 与 keyboard-interactive，禁用 pubkey，并限制密码提示次数。
        ssh_args.extend([
            "-o".to_string(),
            "PreferredAuthentications=password,keyboard-interactive".to_string(),
            "-o".to_string(),
            "PubkeyAuthentication=no".to_string(),
            "-o".to_string(),
            "PasswordAuthentication=yes".to_string(),
            "-o".to_string(),
            "NumberOfPasswordPrompts=1".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
            "-o".to_string(),
            "ConnectTimeout=15".to_string(),
        ]);
    } else {
        ssh_args.extend(["-i".to_string(), key_path]);
    }

    // 如果端口不是 22，添加 -p 参数
    if port != "22" {
        ssh_args.push("-p".to_string());
        ssh_args.push(port);
    }

    ssh_args.push(target);
    if !cmd_to_run.is_empty() {
        ssh_args.push(cmd_to_run);
    }

    let status = if password_mode {
        run_password_ssh(&ssh_cmd, &ssh_args, interactive)
    } else {
        Command::new(ssh_cmd.as_str())
            .args(&ssh_args)
            .status()
            .map(|s| s.code().unwrap_or(1))
            .map_err(|e| e.to_string())
    };

    std::process::exit(status.unwrap_or_else(|e| {
        eprintln!("{}", e);
        1
    }));
}

fn run_password_ssh(ssh_cmd: &str, ssh_args: &[String], interactive: bool) -> Result<i32, String> {
    let askpass_path = env::current_exe()
        .map_err(|e| format!("resolve current exe failed: {}", e))?
        .to_string_lossy()
        .to_string();

    let mut command = Command::new(ssh_cmd);
    // OpenSSH 会执行 SSH_ASKPASS 指向的程序并读取其 stdout 作为密码。
    // 这里指向当前 exe，再用私有环境变量切换到上面的 askpass 分支。
    // DISPLAY 是 OpenSSH askpass 路径的历史要求；值本身不需要对应真实图形会话。
    command
        .args(ssh_args)
        .env("SSH_ASKPASS", askpass_path)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("DISPLAY", "ssh-as-executable")
        .env(ASKPASS_MODE_ENV, "1")
        .env(ASKPASS_TOKEN_ENV, generated::get_askpass_token())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if interactive {
        command.stdin(Stdio::inherit());
    } else {
        // 远程命令场景不使用 PTY，也不从本地 stdin 读数据，保持 key 模式原有的稳定形态。
        command.stdin(Stdio::null());
    }

    command
        .status()
        .map(|status| status.code().unwrap_or(1))
        .map_err(|e| format!("spawn ssh failed: {}", e))
}
