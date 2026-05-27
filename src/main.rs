use std::process::Command;
use std::env;
use std::io::{self, Read};

mod generated;

fn main() {
    let args: Vec<String> = env::args().collect();

    // --help 显示帮助
    if args.len() > 1 && args[1] == "--help" {
        println!("{}", generated::get_help());
        return;
    }

    let key_path = generated::get_key_path();
    let host = generated::get_host();
    let user = generated::get_user();
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

    let cmd_args: Vec<&str> = if cmd_to_run.is_empty() {
        vec![]
    } else {
        vec![cmd_to_run.as_str()]
    };

    let mut ssh_args: Vec<&str> = vec![
        "-i",
        key_path.as_str(),
        "-o",
        ssh_flag.as_str(),
    ];

    // 如果端口不是 22，添加 -p 参数
    if port != "22" {
        ssh_args.push("-p");
        ssh_args.push(port.as_str());
    }

    ssh_args.push(target.as_str());
    ssh_args.extend(cmd_args);

    let status = Command::new(ssh_cmd.as_str())
        .args(&ssh_args)
        .status();

    std::process::exit(status.map(|s| s.code().unwrap_or(1)).unwrap_or(1));
}
