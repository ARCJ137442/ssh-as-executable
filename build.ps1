<#
.SYNOPSIS
    SSH Proxy Exe Factory - 构建碎片化 SSH 代理 exe
.DESCRIPTION
    .\build.ps1 -TargetHost "YOUR_SERVER_IP" -User "root" -KeyPath "~\.ssh\YOUR_KEY_NAME" -Name "server1"
#>

param(
    [string]$Name = "app",
    [string]$TargetHost,
    [string]$User = "root",
    [string]$KeyPath,
    [ValidateSet("key", "password")]
    [string]$AuthMode,
    [string]$Password,
    [string]$Port = "22"
)

# ============================================================
# 读取 .env 文件
# ============================================================
if (Test-Path ".env") {
    foreach ($line in Get-Content ".env") {
        if ($line -notmatch '^([^=]+)=(.*)$') { continue }

        $n, $v = $line.Split('=', 2)
        $n = $n.Trim()
        $v = $v.Trim()
        if (($v.StartsWith('"') -and $v.EndsWith('"')) -or ($v.StartsWith("'") -and $v.EndsWith("'"))) {
            $v = $v.Substring(1, $v.Length - 2)
        }

        switch ($n) {
            "TARGET_NAME" { if (-not $PSBoundParameters.ContainsKey("Name")) { $Name = $v } }
            "TARGET_HOST" { if (-not $PSBoundParameters.ContainsKey("TargetHost")) { $TargetHost = $v } }
            "TARGET_USER" { if (-not $PSBoundParameters.ContainsKey("User")) { $User = $v } }
            "SSH_KEY_PATH" { if (-not $PSBoundParameters.ContainsKey("KeyPath")) { $KeyPath = $v } }
            "SSH_AUTH_MODE" { if (-not $PSBoundParameters.ContainsKey("AuthMode")) { $AuthMode = $v } }
            "SSH_PASSWORD" { if (-not $PSBoundParameters.ContainsKey("Password")) { $Password = $v } }
            "TARGET_PORT" { if (-not $PSBoundParameters.ContainsKey("Port")) { $Port = $v } }
        }
    }
}

# 后备：环境变量
if (-not $PSBoundParameters.ContainsKey("Name") -and $env:TARGET_NAME) { $Name = $env:TARGET_NAME }
if (-not $PSBoundParameters.ContainsKey("TargetHost") -and -not $TargetHost) { $TargetHost = $env:TARGET_HOST }
if (-not $PSBoundParameters.ContainsKey("User") -and $env:TARGET_USER) { $User = $env:TARGET_USER }
if (-not $PSBoundParameters.ContainsKey("KeyPath") -and -not $KeyPath) { $KeyPath = $env:SSH_KEY_PATH }
if (-not $PSBoundParameters.ContainsKey("AuthMode") -and -not $AuthMode) { $AuthMode = $env:SSH_AUTH_MODE }
if (-not $PSBoundParameters.ContainsKey("Password") -and -not $Password) { $Password = $env:SSH_PASSWORD }
if (-not $PSBoundParameters.ContainsKey("Port") -and $env:TARGET_PORT) { $Port = $env:TARGET_PORT }
if (-not $AuthMode) {
    if ($Password) { $AuthMode = "password" } else { $AuthMode = "key" }
}
$AuthMode = $AuthMode.Trim().ToLowerInvariant()
if (-not $KeyPath) { $KeyPath = "" }

# ============================================================
# 验证
# ============================================================
if ($AuthMode -notin @("key", "password")) {
    Write-Host ""
    Write-Host " 错误: SSH_AUTH_MODE 只能是 key 或 password" -ForegroundColor Red
    Write-Host ""
    exit 1
}

if (-not $TargetHost -or ($AuthMode -eq "key" -and -not $KeyPath) -or ($AuthMode -eq "password" -and -not $Password)) {
    Write-Host ""
    Write-Host " 错误: 缺少必需参数" -ForegroundColor Red
    Write-Host ""
    Write-Host " 密钥模式: .\build.ps1 -TargetHost ""IP"" -User ""root"" -KeyPath ""C:\path\to\key"" -Port 22" -ForegroundColor White
    Write-Host " 密码模式: .\build.ps1 -TargetHost ""IP"" -User ""root"" -AuthMode password -Password ""PASSWORD"" -Port 22" -ForegroundColor White
    Write-Host ""
    exit 1
}

$buildEnvNames = @(
    "TARGET_HOST",
    "TARGET_USER",
    "SSH_KEY_PATH",
    "SSH_AUTH_MODE",
    "SSH_PASSWORD",
    "TARGET_PORT",
    "TARGET_NAME"
)
$oldBuildEnv = @{}
foreach ($envName in $buildEnvNames) {
    $oldBuildEnv[$envName] = [System.Environment]::GetEnvironmentVariable($envName, "Process")
}

function Restore-BuildEnvironment {
    foreach ($envName in $buildEnvNames) {
        [System.Environment]::SetEnvironmentVariable($envName, $oldBuildEnv[$envName], "Process")
    }
}

try {
    # 设置环境变量供 cargo build 使用；finally 中会恢复，避免密码留在当前 PowerShell 会话。
    $env:TARGET_HOST = $TargetHost
    $env:TARGET_USER = $User
    $env:SSH_KEY_PATH = $KeyPath
    $env:SSH_AUTH_MODE = $AuthMode
    $env:SSH_PASSWORD = $Password
    $env:TARGET_PORT = $Port
    $env:TARGET_NAME = $Name

# ============================================================
# 确保 dist 目录存在
# ============================================================
    if (-not (Test-Path "dist")) { New-Item -ItemType Directory -Path "dist" | Out-Null }

# ============================================================
# 编译
# ============================================================
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " SSH Proxy Exe Factory" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host " 配置:" -ForegroundColor Yellow
    Write-Host "   主机: $TargetHost" -ForegroundColor Green
    Write-Host "   端口: $Port" -ForegroundColor Green
    Write-Host "   用户: $User" -ForegroundColor Green
    Write-Host "   认证: $AuthMode" -ForegroundColor Green
    if ($AuthMode -eq "key") {
        Write-Host "   密钥: $KeyPath" -ForegroundColor Green
    } else {
        Write-Host "   密码: <已封装>" -ForegroundColor Green
    }
    Write-Host ""

    Write-Host " 编译中..." -ForegroundColor Yellow

# 强制重新执行 build.rs（cargo 增量编译可能跳过）
    if (Test-Path "build.rs") {
        (Get-Item "build.rs").LastWriteTime = Get-Date
    }

    cargo build --release -j 1

    if ($LASTEXITCODE -eq 0) {
        $srcExe = "target\release\app.exe"
        if (-not (Test-Path $srcExe)) { $srcExe = "target\release\ssh-proxy.exe" }
        $dstExe = "dist\$Name.exe"

        if (Test-Path $srcExe) {
            Copy-Item $srcExe $dstExe -Force

            Write-Host ""
            Write-Host " 成功! 输出: dist\$Name.exe" -ForegroundColor Green
            Write-Host ""

            # 安全验证
            Write-Host " 安全验证:" -ForegroundColor Yellow
            $s = strings $srcExe 2>$null
            if ($s -match [regex]::Escape($TargetHost)) {
                Write-Host "   [警告] 主机地址在 strings 中可见" -ForegroundColor Red
            } else {
                Write-Host "   [OK] 主机地址已碎片化" -ForegroundColor Green
            }
            if ($AuthMode -eq "key") {
                if ($s -match [regex]::Escape($KeyPath)) {
                    Write-Host "   [警告] 密钥路径在 strings 中可见" -ForegroundColor Red
                } else {
                    Write-Host "   [OK] 密钥路径已碎片化" -ForegroundColor Green
                }
            } else {
                if ($s -match [regex]::Escape($Password)) {
                    Write-Host "   [警告] 密码在 strings 中可见" -ForegroundColor Red
                } else {
                    Write-Host "   [OK] 密码已碎片化" -ForegroundColor Green
                }
            }

            Write-Host ""
            Write-Host " 使用:" -ForegroundColor Yellow
            Write-Host "   dist\$Name.exe ""whoami""      # 执行命令" -ForegroundColor White
            Write-Host "   dist\$Name.exe ""docker ps""   # 查看容器" -ForegroundColor White
            Write-Host "   dist\$Name.exe                 # 交互式 SSH" -ForegroundColor White
            Write-Host ""
        } else {
            Write-Host " 编译成功但未找到输出 exe: $srcExe" -ForegroundColor Red
            exit 1
        }
    } else {
        $buildExitCode = $LASTEXITCODE
        Write-Host " 编译失败!" -ForegroundColor Red
        exit $buildExitCode
    }
} finally {
    Restore-BuildEnvironment
}
