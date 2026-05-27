//! SSH Proxy Exe Factory - Build Script
//! 数据即算法：所有字符串都通过复杂计算生成

use std::env;
use std::fs;

fn main() {
    let target_host = env::var("TARGET_HOST").expect("TARGET_HOST");
    let target_user = env::var("TARGET_USER").unwrap_or_else(|_| "root".to_string());
    let ssh_key_path = env::var("SSH_KEY_PATH").expect("SSH_KEY_PATH");
    let target_port = env::var("TARGET_PORT").unwrap_or_else(|_| "22".to_string());
    let _exe_name = env::var("TARGET_NAME").unwrap_or_else(|_| "app".to_string());

    // 复杂编码：每个字节由多个操作组合生成
    // 例如: 115 = (12*10) - 5 = 200/2 + 15 = ...
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
        result.push((b, 2, 0));   // b ^ 0 = b (确保最终值正确)

        result
    }

    // 生成使用复杂计算的函数
    fn gen_calc_fn(name: &str, text: &str) -> String {
        let mut s = format!("pub fn {}() -> String {{\n", name);
        s.push_str("    fn c(a: u8, o: u8, b: u8) -> u8 { match o { 0 => a.wrapping_sub(b), 1 => a.wrapping_add(b), _ => a ^ b } }\n");

        // 为每个字符生成计算
        let mut char_exprs = Vec::new();
        for ch in text.chars() {
            let encoded = encode_complex(ch as u8);
            // 使用第一个有效操作，忽略混入的无用操作
            let (a, op, b) = encoded[0];
            char_exprs.push(format!("c({},{},{})", a, op, b));
        }

        s.push_str(&format!("    vec![{}].iter().map(|&v| v as char).collect()\n", char_exprs.join(", ")));
        s.push_str("}\n\n");
        s
    }

    let mut code = String::new();
    code.push_str("// ============================================================\n");
    code.push_str("// DO NOT EDIT - Auto-generated\n");
    code.push_str("// ============================================================\n\n");
    code.push_str("#![allow(dead_code)]\n\n");

    // HOST: IP 分解计算
    let ip_parts: Vec<u8> = target_host.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    code.push_str("pub fn get_host() -> String {\n");
    code.push_str("    fn c(a: u8, o: u8, b: u8) -> u8 { match o { 0 => a.wrapping_sub(b), 1 => a.wrapping_add(b), _ => a ^ b } }\n");
    for (i, &p) in ip_parts.iter().enumerate() {
        let encoded = encode_complex(p);
        let (a, op, b) = encoded[0];
        code.push_str(&format!("    let s{} = c({}, {}, {});\n", i, a, op, b));
    }
    code.push_str("    format!(\"{}.{}.{}.{}\", s0, s1, s2, s3)\n");
    code.push_str("}\n\n");

    // USER
    code.push_str(&gen_calc_fn("get_user", &target_user));

    // KEY_PATH: 分成多个片段打乱
    let chars: Vec<char> = ssh_key_path.chars().collect();
    code.push_str("pub fn get_key_path() -> String {\n");
    code.push_str("    fn c(a: u8, o: u8, b: u8) -> u8 { match o { 0 => a.wrapping_sub(b), 1 => a.wrapping_add(b), _ => a ^ b } }\n");

    // 分成3个chunk，打破连续性
    let chunk_size = (chars.len() + 2) / 3;
    for (ci, chunk) in chars.chunks(chunk_size).enumerate() {
        let exprs: Vec<String> = chunk.iter().map(|&ch| {
            let encoded = encode_complex(ch as u8);
            let (a, op, b) = encoded[0];
            format!("c({},{},{})", a, op, b)
        }).collect();
        code.push_str(&format!("    let g{} = vec![{}];\n", ci, exprs.join(", ")));
    }

    let num_chunks = (chars.len() + chunk_size - 1) / chunk_size;
    // 简单方案：直接 extend
    code.push_str("    let mut all = Vec::new();\n");
    for i in 0..num_chunks {
        code.push_str(&format!("    all.extend(g{});\n", i));
    }
    code.push_str("    all.iter().map(|&v| v as char).collect()\n");
    code.push_str("}\n\n");

    // SSH 命令
    code.push_str(&gen_calc_fn("get_ssh_cmd", "ssh"));

    // SSH 选项
    code.push_str(&gen_calc_fn("get_ssh_flag", "PermitLocalCommand=no"));

    // 端口
    code.push_str(&gen_calc_fn("get_port", &target_port));

    // 帮助文本（纯英文，通过混淆生成）
    let help_text = "Usage: proxy [command]\nExamples:\n  proxy \"whoami\"\n  proxy \"docker ps\"\n  proxy";
    code.push_str(&gen_calc_fn("get_help", &help_text));

    fs::write("src/generated.rs", &code).expect("write");
}
