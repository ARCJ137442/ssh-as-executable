//! SSH Proxy Exe Factory - Build Script
//!
//! Cargo 在编译前运行本脚本，把目标主机、用户、认证方式、密钥路径或密码
//! 转换成 OUT_DIR/generated.rs 里的 Rust 函数。运行时主程序只调用这些函数
//! 恢复配置，不需要在源码树里留下 generated.rs。
//!
//! 注意：这里做的是“可恢复混淆”，目的是提高 strings/简单静态扫描直接拿到
//! 原始配置的成本；它不是不可逆加密，也不能提供硬件密钥或系统凭据库级别的保护。

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    for var in [
        "TARGET_HOST",
        "TARGET_USER",
        "SSH_KEY_PATH",
        "SSH_AUTH_MODE",
        "AUTH_MODE",
        "SSH_PASSWORD",
        "TARGET_PORT",
        "TARGET_NAME",
    ] {
        println!("cargo:rerun-if-env-changed={}", var);
    }

    let target_host = env::var("TARGET_HOST").expect("TARGET_HOST");
    let target_user = env::var("TARGET_USER").unwrap_or_else(|_| "root".to_string());
    let mut ssh_password = env::var("SSH_PASSWORD").unwrap_or_default();
    let auth_mode = env::var("SSH_AUTH_MODE")
        .or_else(|_| env::var("AUTH_MODE"))
        .unwrap_or_else(|_| {
            if ssh_password.is_empty() {
                "key".to_string()
            } else {
                "password".to_string()
            }
        })
        .trim()
        .to_ascii_lowercase();

    if !matches!(auth_mode.as_str(), "key" | "password") {
        panic!("SSH_AUTH_MODE must be key or password");
    }

    let password_mode = auth_mode == "password";
    let ssh_key_path = if password_mode {
        String::new()
    } else {
        env::var("SSH_KEY_PATH").expect("SSH_KEY_PATH")
    };
    let target_port = env::var("TARGET_PORT").unwrap_or_else(|_| "22".to_string());
    let _exe_name = env::var("TARGET_NAME").unwrap_or_else(|_| "app".to_string());

    if password_mode && ssh_password.is_empty() {
        panic!("SSH_PASSWORD is required when SSH_AUTH_MODE=password");
    }
    if !password_mode {
        ssh_password.clear();
    }

    // 复杂编码：每个字节由多个操作组合生成。
    // 目标不是强密码学安全，而是避免敏感字符串作为连续明文进入最终 exe。
    fn encode_complex(b: u8) -> Vec<(u8, u8, u8)> {
        let mut result = Vec::new();
        let remaining = b as i16;

        // 方案1: 分解为多个加减法
        // 115 = 200 - 85 = 100 + 15 = 50 * 2 + 15
        if remaining > 50 {
            let a1 = 200u8;
            let b1 = (200 - remaining) as u8;
            result.push((a1, 0, b1)); // a - b
        } else {
            let a1 = 100u8;
            let b1 = (100 - remaining) as u8;
            result.push((a1, 0, b1));
        }

        // 添加一些无用操作让分析更困难
        result.push((10, 0, 10)); // 10 - 10 = 0 (混入)
        result.push((b, 2, 0)); // b ^ 0 = b (确保最终值正确)

        result
    }

    // 生成使用复杂计算恢复字符串的函数。
    // std::hint::black_box 和逐字节表达式让编译器更难把结果重新折叠为明文常量。
    fn gen_calc_fn(name: &str, text: &str) -> String {
        let mut s = format!("pub fn {}() -> String {{\n", name);
        s.push_str("    #[inline(never)]\n");
        s.push_str("    fn c(a: u8, o: u8, b: u8) -> u8 {\n");
        s.push_str("        let a = std::hint::black_box(a);\n");
        s.push_str("        let o = std::hint::black_box(o);\n");
        s.push_str("        let b = std::hint::black_box(b);\n");
        s.push_str(
            "        match o { 0 => a.wrapping_sub(b), 1 => a.wrapping_add(b), _ => a ^ b }\n",
        );
        s.push_str("    }\n");

        // 为每个字节生成计算，避免非 ASCII 内容被截断
        let mut char_exprs = Vec::new();
        for &b in text.as_bytes() {
            let encoded = encode_complex(b);
            // 使用第一个有效操作，忽略混入的无用操作
            let (a, op, b) = encoded[0];
            char_exprs.push(format!("c({},{},{})", a, op, b));
        }

        s.push_str(&format!(
            "    String::from_utf8(vec![{}]).unwrap_or_default()\n",
            char_exprs.join(", ")
        ));
        s.push_str("}\n\n");
        s
    }

    fn make_askpass_token(parts: &[&str]) -> String {
        // askpass token 用来确认“输出密码”的子进程是由本 exe 启动的。
        // 它是误触发保护，不是安全边界：能逆向 exe 的人仍可能恢复 token 与密码。
        let mut a = 0xcbf29ce484222325u64;
        let mut b = 0x9e3779b97f4a7c15u64;

        for part in parts {
            for byte in part.as_bytes() {
                a ^= *byte as u64;
                a = a.wrapping_mul(0x100000001b3);
                b = b.rotate_left(7) ^ a.wrapping_add(*byte as u64);
            }
            a ^= 0xff;
            a = a.wrapping_mul(0x100000001b3);
            b = b.rotate_left(11) ^ a;
        }

        format!("{:016x}{:016x}", a, b)
    }

    let askpass_token = make_askpass_token(&[
        &target_host,
        &target_user,
        &target_port,
        &auth_mode,
        &ssh_key_path,
        &ssh_password,
    ]);

    let mut code = String::new();
    code.push_str("// ============================================================\n");
    code.push_str("// DO NOT EDIT - Auto-generated\n");
    code.push_str("// ============================================================\n\n");
    // HOST
    code.push_str(&gen_calc_fn("get_host", &target_host));

    // USER
    code.push_str(&gen_calc_fn("get_user", &target_user));

    // AUTH_MODE
    code.push_str(&gen_calc_fn("get_auth_mode", &auth_mode));

    // PASSWORD
    // key 模式下 ssh_password 已被清空，因此不会生成可恢复密码。
    code.push_str(&gen_calc_fn("get_password", &ssh_password));

    // ASKPASS_TOKEN
    code.push_str(&gen_calc_fn("get_askpass_token", &askpass_token));

    // KEY_PATH: 分成多个片段打乱
    let bytes: Vec<u8> = ssh_key_path.as_bytes().to_vec();
    code.push_str("pub fn get_key_path() -> String {\n");
    code.push_str("    #[inline(never)]\n");
    code.push_str("    fn c(a: u8, o: u8, b: u8) -> u8 {\n");
    code.push_str("        let a = std::hint::black_box(a);\n");
    code.push_str("        let o = std::hint::black_box(o);\n");
    code.push_str("        let b = std::hint::black_box(b);\n");
    code.push_str(
        "        match o { 0 => a.wrapping_sub(b), 1 => a.wrapping_add(b), _ => a ^ b }\n",
    );
    code.push_str("    }\n");

    // 分成3个chunk，打破连续性
    let chunk_size = bytes.len().div_ceil(3).max(1);
    for (ci, chunk) in bytes.chunks(chunk_size).enumerate() {
        let exprs: Vec<String> = chunk
            .iter()
            .map(|&b| {
                let encoded = encode_complex(b);
                let (a, op, b) = encoded[0];
                format!("c({},{},{})", a, op, b)
            })
            .collect();
        code.push_str(&format!("    let g{} = vec![{}];\n", ci, exprs.join(", ")));
    }

    let num_chunks = bytes.len().div_ceil(chunk_size);
    if num_chunks == 0 {
        code.push_str("    let all: Vec<u8> = Vec::new();\n");
    } else {
        // 简单方案：直接 extend
        code.push_str("    let mut all = Vec::new();\n");
        for i in 0..num_chunks {
            code.push_str(&format!("    all.extend(g{});\n", i));
        }
    }
    code.push_str("    String::from_utf8(all).unwrap_or_default()\n");
    code.push_str("}\n\n");

    // SSH 命令
    code.push_str(&gen_calc_fn("get_ssh_cmd", "ssh"));

    // SSH 选项
    code.push_str(&gen_calc_fn("get_ssh_flag", "PermitLocalCommand=no"));

    // 端口
    code.push_str(&gen_calc_fn("get_port", &target_port));

    // 帮助文本（纯英文，通过混淆生成）
    let help_text =
        "Usage: proxy [command]\nExamples:\n  proxy \"whoami\"\n  proxy \"docker ps\"\n  proxy";
    code.push_str(&gen_calc_fn("get_help", help_text));

    // 写到 Cargo OUT_DIR，避免 src/generated.rs 出现在仓库里或被误提交。
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("generated.rs"), &code).expect("write generated.rs");
}
