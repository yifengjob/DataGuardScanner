use std::process::Command;

/// 系统环境检查结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnvironmentCheck {
    pub os: String,
    pub os_version: String,
    pub issues: Vec<EnvironmentIssue>,
    pub is_ready: bool,
}

/// 环境问题
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnvironmentIssue {
    pub severity: IssueSeverity,
    pub title: String,
    pub description: String,
    pub solution: String,
    pub download_url: Option<String>,
}

/// 问题严重程度
#[derive(Debug, Clone, serde::Serialize)]
pub enum IssueSeverity {
    Critical,  // 必须解决，无法运行
    Warning,   // 建议解决，但可以运行
}

/// 检查系统环境
pub fn check_environment() -> EnvironmentCheck {
    let os = std::env::consts::OS.to_string();
    let os_version = get_os_version();
    
    let mut issues = Vec::new();
    
    // 根据不同操作系统进行检查
    match os.as_str() {
        "windows" => {
            check_windows_environment(&mut issues);
        }
        "macos" => {
            check_macos_environment(&mut issues);
        }
        "linux" => {
            check_linux_environment(&mut issues);
        }
        _ => {}
    }
    
    let is_ready = !issues.iter().any(|i| matches!(i.severity, IssueSeverity::Critical));
    
    EnvironmentCheck {
        os,
        os_version,
        issues,
        is_ready,
    }
}

/// 获取操作系统版本
fn get_os_version() -> String {
    match std::env::consts::OS {
        "windows" => {
            // 尝试获取 Windows 版本
            if let Ok(version) = Command::new("cmd")
                .args(["/c", "ver"])
                .output()
            {
                String::from_utf8_lossy(&version.stdout).trim().to_string()
            } else {
                "Windows (未知版本)".to_string()
            }
        }
        "macos" => {
            if let Ok(version) = Command::new("sw_vers")
                .arg("-productVersion")
                .output()
            {
                format!("macOS {}", String::from_utf8_lossy(&version.stdout).trim())
            } else {
                "macOS (未知版本)".to_string()
            }
        }
        "linux" => {
            if let Ok(version) = Command::new("uname")
                .arg("-r")
                .output()
            {
                format!("Linux {}", String::from_utf8_lossy(&version.stdout).trim())
            } else {
                "Linux (未知版本)".to_string()
            }
        }
        _ => "Unknown OS".to_string(),
    }
}

/// 检查 Windows 环境
fn check_windows_environment(issues: &mut Vec<EnvironmentIssue>) {
    // 检查 Windows 版本
    let is_windows_7_or_older = is_windows_7_or_older();
    
    if is_windows_7_or_older {
        // 检查 WebView2 是否安装
        if !is_webview2_installed() {
            issues.push(EnvironmentIssue {
                severity: IssueSeverity::Critical,
                title: "缺少 WebView2 运行时".to_string(),
                description: "Windows 7/8/8.1 需要安装 Microsoft Edge WebView2 运行时才能运行此应用。".to_string(),
                solution: "请下载并安装 WebView2 运行时（约 120MB）。\n\n安装完成后，请重新启动应用程序。".to_string(),
                download_url: Some("https://go.microsoft.com/fwlink/p/?LinkId=2124703".to_string()),
            });
        }
    }
    
    // 检查 .NET Framework（如果需要）
    // 注意：Tauri 2.x 不需要 .NET，但某些功能可能需要
    
    // 检查 Visual C++ Redistributable
    if !is_vc_redist_installed() {
        issues.push(EnvironmentIssue {
            severity: IssueSeverity::Warning,
            title: "建议安装 Visual C++ Redistributable".to_string(),
            description: "某些功能可能需要最新版本的 Visual C++ Redistributable。".to_string(),
            solution: "建议安装 Visual C++ Redistributable 以获得最佳兼容性。".to_string(),
            download_url: Some("https://aka.ms/vs/17/release/vc_redist.x64.exe".to_string()),
        });
    }
}

/// 检查 macOS 环境
fn check_macos_environment(issues: &mut Vec<EnvironmentIssue>) {
    // macOS 通常不需要额外检查
    // Tauri 使用系统自带的 WebKit
    
    // 可以检查 macOS 版本是否过旧
    if let Ok(version) = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
    {
        let version_str = String::from_utf8_lossy(&version.stdout).trim().to_string();
        let parts: Vec<&str> = version_str.split('.').collect();
        
        if let Some(major) = parts.first().and_then(|v| v.parse::<u32>().ok()) {
            // macOS 10.15 (Catalina) 是最低要求
            if major < 10 || (major == 10 && parts.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) < 15) {
                issues.push(EnvironmentIssue {
                    severity: IssueSeverity::Critical,
                    title: "macOS 版本过低".to_string(),
                    description: format!("当前版本: macOS {}，需要 macOS 10.15 或更高版本。", version_str),
                    solution: "请升级 macOS 到最新版本。".to_string(),
                    download_url: None,
                });
            }
        }
    }
}

/// 检查 Linux 环境
fn check_linux_environment(issues: &mut Vec<EnvironmentIssue>) {
    // 检查必要的库
    let required_libs = vec![
        ("libwebkit2gtk-4.1", "WebKit2GTK"),
        ("libgtk-3", "GTK 3"),
        ("libsoup-3.0", "libsoup 3"),
    ];
    
    for (lib_name, display_name) in required_libs {
        if !is_library_installed(lib_name) {
            issues.push(EnvironmentIssue {
                severity: IssueSeverity::Critical,
                title: format!("缺少 {}", display_name),
                description: format!("此应用需要 {} 库才能运行。", display_name),
                solution: get_linux_install_command(lib_name),
                download_url: None,
            });
        }
    }
}

/// 检查是否是 Windows 7/8/8.1 或更旧版本
fn is_windows_7_or_older() -> bool {
    // 使用 cmd 检查 Windows 版本
    if let Ok(output) = Command::new("cmd")
        .args(["/c", "ver"])
        .output()
    {
        let version_output = String::from_utf8_lossy(&output.stdout);
        
        // 解析版本号，例如: "Microsoft Windows [版本 6.1.7601]"
        if let Some(start) = version_output.find('[')
            && let Some(end) = version_output[start..].find(']') {
                let version_str = &version_output[start + 1..start + end];
                let version_str = version_str.replace("版本 ", "").trim().to_string();
                let parts: Vec<&str> = version_str.split('.').collect();
                
                if let (Some(major), Some(minor)) = (
                    parts.first().and_then(|v| v.parse::<u32>().ok()),
                    parts.get(1).and_then(|v| v.parse::<u32>().ok())
                ) {
                    // Windows 7 = 6.1, Windows 8 = 6.2, Windows 8.1 = 6.3, Windows 10/11 = 10.0
                    // 返回 true 表示是 Windows 7/8/8.1（需要检查 WebView2）
                    return major == 6 && (1..=3).contains(&minor);
                }
            }
    }
    
    false
}

/// 检查 WebView2 是否已安装
fn is_webview2_installed() -> bool {
    // 使用 reg 命令检查注册表
    let reg_check = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
        ])
        .output();
    
    if let Ok(output) = reg_check
        && output.status.success() {
        return true;
    }
    
    // 检查当前用户的注册表
    let reg_check2 = Command::new("reg")
        .args([
            "query",
            r"HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
        ])
        .output();
    
    if let Ok(output) = reg_check2 {
        return output.status.success();
    }
    
    false
}

/// 检查 Visual C++ Redistributable 是否安装
fn is_vc_redist_installed() -> bool {
    // 使用 reg 命令检查多个可能的版本
    let versions = vec![
        r"HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
        r"HKLM\SOFTWARE\Microsoft\VisualStudio\15.0\VC\Runtimes\x64",
        r"HKLM\SOFTWARE\Microsoft\VisualStudio\16.0\VC\Runtimes\x64",
        r"HKLM\SOFTWARE\Microsoft\VisualStudio\17.0\VC\Runtimes\x64",
    ];
    
    for version in versions {
        if let Ok(output) = Command::new("reg")
            .args(["query", version])
            .output()
            && output.status.success() {
            return true;
        }
    }
    
    false
}

/// 检查 Linux 库是否安装
fn is_library_installed(lib_name: &str) -> bool {
    // 使用 dpkg (Debian/Ubuntu) 或 rpm (RedHat/CentOS) 检查
    if let Ok(_output) = Command::new("dpkg")
        .args(["-l", lib_name])
        .output()
    {
        return true;
    }
    
    if let Ok(_output) = Command::new("rpm")
        .args(["-q", lib_name])
        .output()
    {
        return true;
    }
    
    // 简单检查：尝试使用 pkg-config
    if let Ok(output) = Command::new("pkg-config")
        .arg("--exists")
        .arg(lib_name)
        .output()
    {
        return output.status.success();
    }
    
    false
}

/// 获取 Linux 安装命令
fn get_linux_install_command(lib_name: &str) -> String {
    match lib_name {
        "libwebkit2gtk-4.1" => {
            "Ubuntu/Debian: sudo apt install libwebkit2gtk-4.1-dev\n\
             Fedora: sudo dnf install webkit2gtk4.1-devel\n\
             Arch: sudo pacman -S webkit2gtk-4.1"
        }
        "libgtk-3" => {
            "Ubuntu/Debian: sudo apt install libgtk-3-dev\n\
             Fedora: sudo dnf install gtk3-devel\n\
             Arch: sudo pacman -S gtk3"
        }
        "libsoup-3.0" => {
            "Ubuntu/Debian: sudo apt install libsoup-3.0-dev\n\
             Fedora: sudo dnf install libsoup3-devel\n\
             Arch: sudo pacman -S libsoup3"
        }
        _ => "请参考您的发行版文档安装所需的库。",
    }.to_string()
}
