# ssh-as-executable

**不想暴露 SSH 密钥路径？不想让 Agent 看到密钥内容？用它。**

将 SSH 配置编译进一个独立 exe，用完即删——Agent 只需知道 exe 路径，不知道任何连接细节。

## 解决的问题

| 裸跑 ssh 命令 | 用 ssh-as-executable |
|---------------|---------------------|
| Agent 看到 `ssh -i ~/.ssh/id_ed25519 root@host` | Agent 只看到 `proxy.exe "whoami"` |
| 密钥路径暴露在进程参数 | 密钥完全封装在 exe 内部 |
| 每次都要处理 SSH 提示 | 一次性配置，永久有效 |
| 删除/撤销麻烦 | 删 exe 即可吊销 |

## 安全特性

- **密钥内容永不暴露**：密钥路径和内容在 exe 内部计算生成
- **静态分析无效**：字符串搜索找不到明文 IP/用户名/密钥路径
- **subAgent 无法识别**：测试表明即使无上下文分析也无法识别为 SSH 工具
- **即时吊销**：删除 exe = 吊销访问权限，无需修改服务器

## 快速开始

### 编译

```bash
# 设置环境变量
export TARGET_HOST="YOUR_SERVER_IP"
export TARGET_USER="root"
export SSH_KEY_PATH="$HOME/.ssh/YOUR_KEY_NAME"

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
| `src/generated.rs` | 自动生成 - 包含混淆后的配置 |
| `src/main.rs` | SSH 代理逻辑 |
| `build.rs` | 编译时生成混淆代码 |

## 工作原理

```
配置(IP/用户名/密钥) → build.rs 混淆 → generated.rs → 编译 → exe
                                    ↓
                        算法计算生成，非数据存储
```

1. `build.rs` 在编译时读取环境变量
2. 将配置转换为算术表达式（如 `c(200,0,79)` = 121）
3. 生成的 `generated.rs` 在运行时计算得出真实值
4. 关键秘密从不以明文形式存在于二进制

## 构建配置

### 环境变量

| 变量 | 必需 | 默认值 | 说明 |
|------|------|--------|------|
| `TARGET_HOST` | 是 | - | 目标服务器 IP |
| `TARGET_USER` | 否 | root | SSH 用户名 |
| `SSH_KEY_PATH` | 是 | - | 本地私钥路径 |
| `TARGET_PORT` | 否 | 22 | SSH 端口 |

### 示例

```bash
TARGET_HOST="YOUR_SERVER_IP" TARGET_USER="root" SSH_KEY_PATH="$HOME/.ssh/YOUR_KEY_NAME" TARGET_PORT="22" cargo build --release
```

## 安全验证

```bash
# 搜索 IP - 无结果
strings target/release/app.exe | grep "YOUR_SERVER_IP"

# 搜索密钥路径 - 无结果
strings target/release/app.exe | grep "YOUR_KEY_NAME"

# 搜索 ssh 相关 - 无结果
strings target/release/app.exe | grep -i "ssh"
```

## 项目结构

```
ssh-as-executable/
├── build.rs           # 编译时混淆代码生成
├── Cargo.toml        # 项目配置
├── src/
│   ├── main.rs      # SSH 代理入口
│   └── generated.rs # 自动生成（勿手动编辑）
└── README.md
```

## 使用场景

- **AI Agent 集成**：Agent 通过 exe 执行远程命令，不知道真实服务器地址
- **安全分发**：exe 可以分发给第三方，无法从中提取连接信息
- **临时访问**：编译一个短期访问 exe，用完删除即可

## 限制

- 目标 IP 和用户名在 SSH 握手时会暴露给服务器
- 密钥路径虽然隐藏，但 `ssh -i` 参数仍会出现在进程树中（密钥内容不会）
- 需要本地有 `ssh` 命令且密钥已授权

## 名称说明

`ssh-as-executable` 表达的是"把 SSH 配置当作可执行文件的一部分"——配置编译进 exe，运行时像普通命令一样调用。

---

*编译产物仅供本地使用，请遵守相关法律法规。*
