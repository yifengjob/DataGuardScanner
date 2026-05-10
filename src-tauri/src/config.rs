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
pub const DEFAULT_CONCURRENCY_MAX: usize = 8;

/// 默认并发数最小值
pub const DEFAULT_CONCURRENCY_MIN: usize = 2;

// ==================== 事件通道配置 ====================

/// 事件通道缓冲区大小
pub const EVENT_CHANNEL_BUFFER_SIZE: usize = 500;

// ==================== 日志配置 ====================

/// 日志节流间隔（毫秒）
pub const LOG_THROTTLE_MS: u64 = 100;

/// 日志数组最大长度（防止内存泄漏）
pub const MAX_LOG_ENTRIES: usize = 1000;

/// 初始阶段允许快速通过日志的时间窗口（秒）
pub const INITIAL_LOG_PHASE_SECS: u64 = 3;

// ==================== 超时配置 ====================

/// 扫描总超时时间（秒）- 1小时
pub const SCAN_TIMEOUT_SECS: u64 = 3600;

/// 停滞检测间隔（秒）
pub const STAGNATION_CHECK_INTERVAL_SECS: u64 = 5;

/// 停滞警告阈值（秒）- 30秒无进展发出警告
pub const STAGNATION_WARNING_THRESHOLD_SECS: u64 = 30;

/// 停滞强制停止阈值（秒）- 120秒无进展强制结束
pub const STAGNATION_FORCE_STOP_THRESHOLD_SECS: u64 = 120;

/// 【新增】单文件解析超时（秒）- 防止单个文件卡住整个扫描
pub const SINGLE_FILE_PARSE_TIMEOUT_SECS: u64 = 30;

// ==================== 动态超时配置 ====================

/// 小文件超时（< 1MB）
pub const TIMEOUT_SMALL_FILE_SECS: u64 = 60;

/// 中等文件超时（1-10MB）
pub const TIMEOUT_MEDIUM_FILE_SECS: u64 = 60;

/// 大文件超时（10-50MB）
pub const TIMEOUT_LARGE_FILE_SECS: u64 = 120;

/// 超大文件超时（> 50MB）
pub const TIMEOUT_HUGE_FILE_SECS: u64 = 180;

// ==================== 进度更新配置 ====================

/// 进度更新频率（每 N 个文件更新一次）
pub const PROGRESS_UPDATE_INTERVAL: u64 = 10;

// ==================== 窗口配置 ====================

/// 窗口最小宽度（逻辑像素）
pub const WINDOW_MIN_WIDTH: u32 = 1000;

/// 窗口最小高度（逻辑像素）
pub const WINDOW_MIN_HEIGHT: u32 = 600;

/// 窗口目标尺寸比例（屏幕的百分比）
pub const WINDOW_TARGET_RATIO: f64 = 0.8;

/// 窗口居中延迟（毫秒）
pub const WINDOW_CENTER_DELAY_MS: u64 = 100;

// ==================== 预览配置 ====================

/// 默认预览最大字节数（200KB）
pub const DEFAULT_PREVIEW_MAX_BYTES: usize = 200 * 1024;

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

/// Office 文档扩展名（包括 WPS）
pub const OFFICE_FILE_EXTENSIONS: &[&str] = &[
    // Word 文档
    "docx", "doc", "wps",
    // Excel 表格
    "xlsx", "xls", "et",
    // PowerPoint 演示
    "pptx", "ppt", "dps",
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
