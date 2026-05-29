#!/usr/bin/env python3
"""
SSH-as-Executable 交互式构建脚本
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path


def main():
    print("=" * 50)
    print(" SSH-as-Executable 构建脚本")
    print("=" * 50)
    print()

    # 检查 Rust 环境
    try:
        subprocess.run(["cargo", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("错误: 未找到 cargo，请先安装 Rust")
        print("访问 https://rustup.rs 安装")
        sys.exit(1)

    print("请输入配置信息:")
    print()

    # 目标服务器
    target_host = input("目标服务器 IP: ").strip()
    while not target_host:
        target_host = input("目标服务器 IP (必填): ").strip()

    # SSH 用户名
    default_user = os.environ.get("TARGET_USER", "root")
    user_input = input(f"SSH 用户名 [{default_user}]: ").strip()
    target_user = user_input if user_input else default_user

    # SSH 端口
    default_port = os.environ.get("TARGET_PORT", "22")
    port_input = input(f"SSH 端口 [{default_port}]: ").strip()
    target_port = port_input if port_input else default_port

    # SSH 私钥路径
    default_key = os.environ.get("SSH_KEY_PATH", "~/.ssh/id_ed25519")
    key_input = input(f"SSH 私钥路径 [{default_key}]: ").strip()
    ssh_key_path = key_input if key_input else default_key

    # exe 输出名字
    default_name = os.environ.get("TARGET_NAME", "ssh-proxy")
    name_input = input(f"exe 输出名字 [{default_name}]: ").strip()
    exe_name = name_input if name_input else default_name
    if not exe_name.endswith(".exe"):
        exe_name += ".exe"

    # 确认配置
    print()
    print("-" * 50)
    print("配置确认:")
    print(f"  目标服务器: {target_host}")
    print(f"  SSH 端口:   {target_port}")
    print(f"  SSH 用户名: {target_user}")
    print(f"  私钥路径:   {ssh_key_path}")
    print(f"  exe 名字:   {exe_name}")
    print("-" * 50)
    print()

    confirm = input("确认构建? (Y/n): ").strip().lower()
    if confirm and confirm != 'y':
        print("取消构建")
        sys.exit(0)

    # 构建
    print()
    print("正在构建...")
    print()

    # 强制重新执行 build.rs（cargo 增量编译可能跳过）
    build_rs = Path(__file__).parent / "build.rs"
    build_rs.touch()

    env = os.environ.copy()
    env["TARGET_HOST"] = target_host
    env["TARGET_USER"] = target_user
    env["SSH_KEY_PATH"] = ssh_key_path
    env["TARGET_PORT"] = target_port
    env["TARGET_NAME"] = exe_name

    result = subprocess.run(
        ["cargo", "build", "--release", "-j", "1"],
        env=env,
        cwd=Path(__file__).parent,
    )

    if result.returncode != 0:
        print()
        print("构建失败!")
        sys.exit(1)

    # 查找 exe 并复制到项目根目录
    src_exe = Path(__file__).parent / "target" / "release" / "app.exe"
    if not src_exe.exists():
        src_exe = Path(__file__).parent / "target" / "release" / "ssh-proxy.exe"

    dst_exe = Path(__file__).parent / exe_name
    shutil.copy2(src_exe, dst_exe)

    # 清理敏感文件：生成的混淆代码（含凭据）
    generated_rs = Path(__file__).parent / "src" / "generated.rs"
    if generated_rs.exists():
        generated_rs.unlink()
        print("  ✓ src/generated.rs 已删除（敏感混淆数据已清除）")

    print()
    print("=" * 50)
    print(" 构建成功!")
    print("=" * 50)
    print()
    print(f"  exe 路径: {dst_exe}")
    print()
    print("使用方式:")
    print(f"  {dst_exe} \"whoami\"")
    print(f"  {dst_exe} \"docker ps\"")
    print(f"  {dst_exe}")
    print()

    # 安全验证提示
    print("安全验证:")
    print(f"  strings \"{dst_exe}\" | findstr \"{target_host}\"")
    print("  (无输出 = 安全)")
    print()

    # 等待用户确认退出
    input("按回车键退出...")


if __name__ == "__main__":
    main()
