#![allow(dead_code)]
/// 扫描器配置常量
/// 集中管理所有魔法数字、硬编码值，便于维护和调整

// ==================== 文件大小限制 ====================

/// 字节到 MB 的转换因子
pub const BYTES_TO_MB: u64 = 1024 * 1024;

/// 字节到 GB 的转换因子
pub const BYTES_TO_GB: f64 = 1024.0 * 1024.0 * 1024.0;

/// 默认最大文件大小（MB）
pub const DEFAULT_MAX_FILE_SIZE_MB: u64 = 50;

/// 默认最大 PDF 文件大小（MB）
pub const DEFAULT_MAX_PDF_SIZE_MB: u64 = 100;

// ==================== 并发控制 ====================

/// 每个 Worker 预估内存占用（GB）
pub const MEMORY_PER_WORKER_GB: f64 = 0.15;

/// 并发数绝对最大值
pub const CONCURRENCY_ABSOLUTE_MAX: usize = 8;

/// 并发数计算时使用的安全内存比例
pub const CONCURRENCY_MEMORY_RATIO: f64 = 0.7;

/// 默认并发数的 CPU 核心数比例
pub const DEFAULT_CONCURRENCY_CPU_RATIO: f64 = 0.5;

/// 默认并发数最大值
pub const DEFAULT_CONCURRENCY_MAX: usize = 6;

/// 默认并发数最小值
pub const DEFAULT_CONCURRENCY_MIN: usize = 2;

// ==================== 事件通道配置 ====================

/// 事件通道缓冲区大小
pub const EVENT_CHANNEL_BUFFER_SIZE: usize = 500;

// ==================== 批量结果发送配置 ====================

/// 批量发送大小（累积多少个结果后发送）
pub const RESULT_BATCH_SIZE: usize = 50;

/// 批量发送超时时间（毫秒）- 即使未达到批量大小，超时后也会发送
pub const RESULT_BATCH_TIMEOUT_MS: u64 = 200;

// ==================== 智能调度器配置 ====================

/// 【新增】小文件阈值（MB）- 小于此值视为小文件
#[allow(dead_code)]
pub const SCHEDULER_SMALL_FILE_THRESHOLD_MB: f64 = 1.0;

/// 【新增】中等文件阈值（MB）- 小于此值视为中等文件
pub const SCHEDULER_MEDIUM_FILE_THRESHOLD_MB: f64 = 10.0;

/// 【新增】大文件阈值（MB）- 大于等于此值视为大文件
pub const SCHEDULER_LARGE_FILE_THRESHOLD_MB: f64 = 50.0;

/// 【新增】超大文件阈值（MB）- 大于此值视为超大文件
pub const SCHEDULER_ULTRA_LARGE_THRESHOLD_MB: f64 = 100.0;

/// 【新增】大文件最大并发数 - 限制同时处理的大文件数量
pub const LARGE_FILE_MAX_CONCURRENCY: usize = 2;

/// 【新增】类型互斥超时时间（毫秒）- 如果超过此时间找不到不同类型，允许同类型
pub const TYPE_MUTEX_TIMEOUT_MS: u64 = 5000;

// ==================== 日志配置 ====================

// --- 日志级别定义 ---

/// 日志级别枚举（与Electron版本对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// --- 日志级别配置 ---

/// 输出到文件的日志级别
/// - dev模式: DEBUG（看到所有调试信息）
/// - release模式: INFO（只记录重要信息）
#[cfg(debug_assertions)]
pub const LOG_FILE_LEVEL: LogLevel = LogLevel::Debug;

#[cfg(not(debug_assertions))]
pub const LOG_FILE_LEVEL: LogLevel = LogLevel::Info;

/// 输出到前端的日志级别
/// - dev模式: DEBUG（前端控制台显示所有日志）
/// - release模式: WARN（只显示警告和错误）
#[cfg(debug_assertions)]
pub const LOG_FRONTEND_LEVEL: LogLevel = LogLevel::Debug;

#[cfg(not(debug_assertions))]
pub const LOG_FRONTEND_LEVEL: LogLevel = LogLevel::Info;

// --- 日志输出开关配置 ---

/// 是否启用文件日志输出
pub const LOG_ENABLE_FILE: bool = true;

/// 是否启用前端日志输出（IPC 通信）
pub const LOG_ENABLE_FRONTEND: bool = true;

// --- 日志频率控制配置 ---

/// 错误日志输出间隔（每 N 个错误输出一条）
pub const ERROR_LOG_INTERVAL: usize = 50;

/// 结果日志计数间隔（每 N 个结果输出一条）
pub const RESULT_LOG_COUNT_INTERVAL: usize = 100;

/// 结果日志时间间隔（毫秒）
pub const RESULT_LOG_TIME_INTERVAL: u64 = 1000; // 1秒

// --- 日志保留策略 ---

/// 日志文件保留天数
pub const LOG_RETENTION_DAYS: u64 = 30;

// --- 其他日志配置 ---

/// 日志节流间隔（毫秒）
pub const LOG_THROTTLE_MS: u64 = 100;

/// 日志数组最大长度（防止内存泄漏）
pub const MAX_LOG_ENTRIES: usize = 1000;

/// 初始阶段允许快速通过日志的时间窗口（秒）
pub const INITIAL_LOG_PHASE_SECS: u64 = 3;

// ==================== 超时配置 ====================

/// 扫描总超时时间（秒）- 1小时
pub const SCAN_TIMEOUT_SECS: u64 = 3600;

/// 停滞检测间隔（秒）- 【优化】缩短为1秒以提高响应速度
pub const STAGNATION_CHECK_INTERVAL_SECS: u64 = 1;

/// 停滞警告阈值（秒）- 【优化】缩短为15秒以便更早发现问题
pub const STAGNATION_WARNING_THRESHOLD_SECS: u64 = 15;

/// 停滞强制停止阈值（秒）- 120秒无进展强制结束
pub const STAGNATION_FORCE_STOP_THRESHOLD_SECS: u64 = 120;

/// 【新增】单文件解析超时（秒）- 防止单个文件卡住整个扫描
pub const SINGLE_FILE_PARSE_TIMEOUT_SECS: u64 = 30;

// ==================== 动态超时配置 ====================

/// 【新增】动态超时计算参数 - 最小超时（秒）
pub const DYNAMIC_TIMEOUT_MIN_SECS: f64 = 30.0;

/// 【新增】动态超时计算参数 - 最大超时（秒）
pub const DYNAMIC_TIMEOUT_MAX_SECS: f64 = 600.0;

/// 【新增】动态超时计算参数 - 衰减系数（控制曲线形状）
pub const DYNAMIC_TIMEOUT_DECAY_K: f64 = 10.0;

/// 【新增】不同文件类型的超时倍数 - PDF
pub const TIMEOUT_MULTIPLIER_PDF: f64 = 1.5;

/// 【新增】不同文件类型的超时倍数 - Word文档
pub const TIMEOUT_MULTIPLIER_WORD: f64 = 1.3;

/// 【新增】不同文件类型的超时倍数 - Excel表格
pub const TIMEOUT_MULTIPLIER_EXCEL: f64 = 1.4;

/// 【新增】不同文件类型的超时倍数 - PowerPoint演示
pub const TIMEOUT_MULTIPLIER_POWERPOINT: f64 = 1.4;

/// 【新增】不同文件类型的超时倍数 - 压缩文件
pub const TIMEOUT_MULTIPLIER_ARCHIVE: f64 = 1.2;

/// 【新增】不同文件类型的超时倍数 - 其他文件
pub const TIMEOUT_MULTIPLIER_DEFAULT: f64 = 1.0;

// ==================== 进度更新配置 ====================

/// 进度更新频率（每 N 个文件更新一次）- 【优化】默认值
pub const PROGRESS_UPDATE_INTERVAL: u64 = 10;

/// 【新增】初始阶段快速更新的文件数阈值（前N个文件每个都更新）
pub const PROGRESS_INITIAL_FAST_COUNT: u64 = 50;

/// 【新增】大量文件时的降频阈值（超过N个文件后降低更新频率）
pub const PROGRESS_MASSIVE_FILE_THRESHOLD: u64 = 10000;

/// 【新增】大量文件时的更新间隔（每N个文件更新一次）
pub const PROGRESS_MASSIVE_UPDATE_INTERVAL: u64 = 100;

// ==================== 窗口配置 ====================

/// 窗口最小宽度（逻辑像素）
pub const WINDOW_MIN_WIDTH: u32 = 1024;

/// 窗口最小高度（逻辑像素）
pub const WINDOW_MIN_HEIGHT: u32 = 700;

/// 窗口目标尺寸比例（屏幕的百分比）
pub const WINDOW_TARGET_RATIO: f64 = 0.85;

/// 窗口居中延迟（毫秒）
pub const WINDOW_CENTER_DELAY_MS: u64 = 100;

// ==================== 预览配置 ====================

/// 默认预览最大字节数（200KB）
pub const DEFAULT_PREVIEW_MAX_BYTES: usize = 200 * 1024;

// ==================== 流式处理配置 ====================

/// 【新增】流式处理分块大小：5MB（与Electron版对齐）
pub const STREAM_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// 【新增】流式处理重叠区大小：200字符（最大敏感词长度 × 2）
pub const STREAM_OVERLAP_SIZE: usize = 200;

// ==================== 系统目录配置 ====================

/// Windows C盘系统目录列表
pub const WINDOWS_SYSTEM_DIRS_C_DRIVE: &[&str] = &[
    "C:\\Windows",
    "C:\\WinNT",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
    "C:\\Recovery",
    "C:\\PerfLogs",
    "C:\\Boot",
    "C:\\EFI",
    "C:\\pagefile.sys",
    "C:\\hiberfil.sys",
    "C:\\swapfile.sys",
];

/// macOS 系统目录列表
pub const MACOS_SYSTEM_DIRS: &[&str] = &[
    "/System",
    "/Library",
    "/private",
    "/Applications",
    "/Applications/Utilities",
    "/dev",
    "/Volumes",
];

/// Linux 系统目录列表
pub const LINUX_SYSTEM_DIRS: &[&str] = &[
    "/proc",
    "/sys",
    "/dev",
    "/dev/pts",
    "/run",
    "/var/run",
    "/var/lock",
    "/bin",
    "/sbin",
    "/lib",
    "/lib64",
    "/usr",
    "/boot",
    "/initrd",
    "/vmlinuz",
    "/mnt",
    "/media",
    "/cdrom",
    "/opt",
    "/srv",
    "/snap",
    "/var/lib/snapd",
    "/var/lib/flatpak",
];

/// Windows 其他磁盘系统目录模板
pub const WINDOWS_OTHER_DRIVES_SYSTEM_DIRS: &[&str] = &[
    "{}:\\Windows",
    "{}:\\Program Files",
    "{}:\\Program Files (x86)",
    "{}:\\ProgramData",
];

/// 忽略的目录名称（所有平台通用）
pub const IGNORE_DIR_NAMES: &[&str] = &[
    // 版本控制和开发工具
    "node_modules", ".git", ".svn", ".hg", ".bzr", "_darcs",
    // IDE 和编辑器
    ".vscode", ".idea", ".eclipse", ".settings", ".project",
    // 包管理器
    ".npm", ".yarn", ".pnpm-store", "bower_components",
    // 操作系统隐藏文件和目录
    "System Volume Information",
    ".Spotlight-V100", ".fseventsd", ".DS_Store",
    "lost+found",
];

// ==================== 敏感信息检测类型 ====================

/// 默认启用的敏感信息检测类型
pub const DEFAULT_SENSITIVE_TYPES: &[&str] = &[
    "person_id",
    "phone",
    "email",
    "bank_card",
    "address",
    "ip_address",
    "password",
];

// ==================== 文件类型分类 ====================

/// 不支持预览的文件类型（压缩文件等）
pub const UNSUPPORTED_PREVIEW_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz",
];

/// 文本文件扩展名列表
pub const TEXT_FILE_EXTENSIONS: &[&str] = &[
    // 纯文本和配置文件
    "txt", "log", "md", "ini", "conf", "cfg", "env",
    // 编程语言
    "js", "ts", "py", "java", "c", "cpp", "go", "rs",
    "php", "rb", "swift",
    // Web 前端
    "html", "htm",
    // 脚本
    "sh", "cmd", "bat",
    // 数据格式
    "csv", "json", "xml", "yaml", "yml", "properties", "toml",
];

/// PDF 文件扩展名
pub const PDF_EXTENSIONS: &[&str] = &[
    "pdf",
];

/// Office 文档扩展名（包括 WPS、OpenDocument 和 RTF）
pub const OFFICE_FILE_EXTENSIONS: &[&str] = &[
    // Word 文档
    "docx", "doc", "wps",
    // Excel 表格
    "xlsx", "xls", "et",
    // PowerPoint 演示
    "pptx", "ppt", "dps",
    // OpenDocument 格式
    "odt", "ods", "odp",
    // 【新增】RTF 富文本
    "rtf",
];

// ==================== 文件处理器映射 ====================

/// 文件处理器类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileHandler {
    Text,
    Pdf,
    Office,
}

impl FileHandler {
    /// 根据文件扩展名获取对应的处理器
    pub fn from_extension(ext: &str) -> Option<FileHandler> {
        if TEXT_FILE_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Text)
        } else if PDF_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Pdf)
        } else if OFFICE_FILE_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Office)
        } else {
            None
        }
    }
}

// ==================== 【新增】配置持久化函数 ====================

use std::fs;
use std::path::PathBuf;

/// 应用标识符（从 tauri.conf.json 中的 identifier 读取）
/// 【严格模式】编译时必须设置，否则编译失败，确保完全一致性
pub const APP_IDENTIFIER: &str = env!("TAURI_APP_IDENTIFIER");

/// 获取配置文件路径
/// 【优化】使用 Tauri API 获取应用配置目录，自动使用 identifier 作为目录名
pub fn get_config_file_path() -> PathBuf {
    // 优先使用 Tauri 提供的配置目录（会自动使用 identifier）
    // macOS: ~/Library/Application Support/{identifier}/
    // Windows: %APPDATA%/{identifier}/
    // Linux: ~/.config/{identifier}/
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join(APP_IDENTIFIER).join("config.json")
    } else {
        // 降级方案：当前目录
        PathBuf::from("config.json")
    }
}

/// 加载应用配置
pub fn load_app_config() -> Result<crate::models::AppConfig, String> {
    let config_path = get_config_file_path();
    
    if !config_path.exists() {
        return Ok(crate::models::AppConfig::default());
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("解析配置文件失败: {}", e))
}

/// 保存应用配置
pub fn save_app_config(config: &crate::models::AppConfig) -> Result<(), String> {
    let config_path = get_config_file_path();
    
    // 确保目录存在
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    
    fs::write(&config_path, content)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_identifier_constant() {
        // 验证 APP_IDENTIFIER 常量是否正确设置
        println!("APP_IDENTIFIER: {}", APP_IDENTIFIER);
        assert!(!APP_IDENTIFIER.is_empty());
        assert_eq!(APP_IDENTIFIER, "com.content.inspector");
    }

    #[test]
    fn test_config_file_path() {
        let path = get_config_file_path();
        println!("Config path: {:?}", path);
        
        // 验证路径包含 identifier
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("com.content.inspector"));
        assert!(path_str.ends_with("config.json"));
    }
}
