use std::process::Command;
use std::env;

mod generated;

fn main() {
    let args: Vec<String> = env::args().collect();

    // 无帮助信息，不泄露任何提示

    let key_path = generated::get_key_path();
    let host = generated::get_host();
    let user = generated::get_user();
    let ssh_cmd = generated::get_ssh_cmd();
    let ssh_flag = generated::get_ssh_flag();

    // 解析命令行参数
    let (target, cmd_args) = if args.len() < 2 {
        (format!("{}@{}", user, host), vec![])
    } else if args.len() == 2 {
        let a = &args[1];
        if a.contains('@') {
            (a.clone(), vec![])
        } else {
            (format!("{}@{}", user, host), vec![a.clone()])
        }
    } else {
        let a = &args[1];
        if a.contains('@') {
            (a.clone(), args[2..].to_vec())
        } else {
            (format!("{}@{}", user, a.clone()), args[2..].to_vec())
        }
    };

    let mut ssh_args: Vec<&str> = vec![
        "-i",
        key_path.as_str(),
        "-o",
        ssh_flag.as_str(),
        target.as_str(),
    ];
    ssh_args.extend(cmd_args.iter().map(|s| s.as_str()));

    let status = Command::new(ssh_cmd.as_str())
        .args(&ssh_args)
        .status();

    std::process::exit(status.map(|s| s.code().unwrap_or(1)).unwrap_or(1));
}
