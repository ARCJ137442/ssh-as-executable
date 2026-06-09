# ssh-as-executable

**不想暴露 SSH 密钥路径或密码？不想让 Agent 看到连接细节？用它。**

将 SSH 配置编译进一个独立 exe，用完即删——Agent 只需知道 exe 路径，不知道任何连接细节。

## 解决的问题

| 裸跑 ssh 命令 | 用 ssh-as-executable |
|---------------|---------------------|
| Agent 看到 `ssh -i ~/.ssh/id_ed25519 root@host` 或输入密码 | Agent 只看到 `proxy.exe "whoami"` |
| 密钥路径/密码暴露在命令或交互里 | 连接配置封装在 exe 内部 |
| 每次都要处理 SSH 提示 | 一次性配置，永久有效 |
| 删除/撤销麻烦 | 删 exe 即可吊销 |

## 安全特性

- **Agent 调用命令不含连接秘密**：Agent 只运行 exe，看不到密钥路径或密码
- **静态字符串搜索难以命中**：直接搜索找不到明文 IP/用户名/密钥路径/密码
- **黑盒分发更简单**：subAgent 或同事只需要运行 exe，不需要知道 SSH 参数或额外输入密码
- **即时吊销**：删除 exe = 吊销访问权限，无需修改服务器

## 快速开始

### 编译

```bash
# 密钥模式
export TARGET_HOST="YOUR_SERVER_IP"
export TARGET_USER="root"
export SSH_AUTH_MODE="key"
export SSH_KEY_PATH="$HOME/.ssh/YOUR_KEY_NAME"

# 编译
cargo build --release
```

```bash
# 密码模式
export TARGET_HOST="YOUR_SERVER_IP"
export TARGET_USER="root"
export SSH_AUTH_MODE="password"
export SSH_PASSWORD="YOUR_PASSWORD"

# 编译
cargo build --release
```

### 使用

```bash
# 执行远程命令
./target/release/app.exe "whoami"
./target/release/app.exe "docker ps"

# 从 stdin 读取命令执行
echo "whoami" | ./target/release/app.exe --stdin
./target/release/app.exe --stdin < cmd.sh

# 交互式 SSH
./target/release/app.exe
```

## 编译产物

| 文件 | 说明 |
|------|------|
| `src/main.rs` | SSH 代理逻辑 |
| `build.rs` | 编译时在 Cargo `OUT_DIR` 生成混淆代码 |

## 工作原理

```
配置(IP/用户名/密钥路径或密码) → build.rs 混淆 → Cargo OUT_DIR/generated.rs → 编译 → exe
                                             ↓
                                 算法计算生成，非数据存储
```

1. `build.rs` 在编译时读取环境变量
2. 将配置转换为算术表达式（如 `c(200,0,79)` = 121）
3. 生成代码位于 Cargo `OUT_DIR`，运行时计算得出真实值
4. key 模式用普通 OpenSSH 进程执行 `ssh -i`，不使用 PTY
5. password 模式用 OpenSSH `SSH_ASKPASS`，把当前 exe 临时作为 askpass helper 返回密码，也不使用 PTY
6. 关键秘密不以连续明文字符串形式存在于二进制

## 构建配置

### 环境变量

| 变量 | 必需 | 默认值 | 说明 |
|------|------|--------|------|
| `TARGET_HOST` | 是 | - | 目标服务器 IP |
| `TARGET_USER` | 否 | root | SSH 用户名 |
| `SSH_AUTH_MODE` | 否 | key | 认证方式：`key` 或 `password` |
| `SSH_KEY_PATH` | key 模式是 | - | 本地私钥路径 |
| `SSH_PASSWORD` | password 模式是 | - | SSH 密码 |
| `TARGET_PORT` | 否 | 22 | SSH 端口 |

### 示例

```bash
TARGET_HOST="YOUR_SERVER_IP" TARGET_USER="root" SSH_AUTH_MODE="key" SSH_KEY_PATH="$HOME/.ssh/YOUR_KEY_NAME" TARGET_PORT="22" cargo build --release
TARGET_HOST="YOUR_SERVER_IP" TARGET_USER="root" SSH_AUTH_MODE="password" SSH_PASSWORD="YOUR_PASSWORD" TARGET_PORT="22" cargo build --release
```

Windows 下也可以使用 `build.ps1`，输出会复制到 ignored 的 `dist/` 目录：

```powershell
.\build.ps1 -Name "server-key" -TargetHost "YOUR_SERVER_IP" -User "root" -AuthMode key -KeyPath "C:\path\to\key" -Port 22
.\build.ps1 -Name "server-password" -TargetHost "YOUR_SERVER_IP" -User "root" -AuthMode password -Password "YOUR_PASSWORD" -Port 22
```

## 安全验证

```bash
# 搜索 IP - 无结果
strings target/release/app.exe | grep "YOUR_SERVER_IP"

# 搜索密钥路径 - 无结果
strings target/release/app.exe | grep "YOUR_KEY_NAME"

# 搜索密码 - 无结果
strings target/release/app.exe | grep "YOUR_PASSWORD"

# 搜索用户名 - 无结果
strings target/release/app.exe | grep "root"
```

## 项目结构

```
ssh-as-executable/
├── build.rs           # 编译时混淆代码生成
├── Cargo.toml        # 项目配置
├── src/
│   └── main.rs      # SSH 代理入口
└── README.md
```

## 使用场景

- **AI Agent 集成**：Agent 通过 exe 执行远程命令，不知道真实服务器地址
- **安全分发**：exe 可以分发给第三方，无法从中提取连接信息
- **临时访问**：编译一个短期访问 exe，用完删除即可

## 限制

- 目标 IP 和用户名在 SSH 握手时会暴露给服务器
- 密钥模式下 `ssh -i` 参数仍会出现在进程树中（密钥内容不会）
- 密码模式依赖 OpenSSH `SSH_ASKPASS` 机制；目标机器必须允许密码或 keyboard-interactive 登录
- 密码模式的密码可被 exe 在运行时恢复；它提高静态逆向成本，但不等同于硬件密钥或系统凭据库
- 需要本地有 `ssh` 命令

## 名称说明

`ssh-as-executable` 表达的是"把 SSH 配置当作可执行文件的一部分"——配置编译进 exe，运行时像普通命令一样调用。

---

*编译产物仅供本地使用，请遵守相关法律法规。*
