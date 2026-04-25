use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 目录树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryNode {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub has_children: bool,
    pub children: Option<Vec<DirectoryNode>>,
}

/// 扫描配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub selected_paths: Vec<String>,
    pub selected_extensions: Vec<String>,
    pub enabled_sensitive_types: Vec<String>,
    pub ignore_dir_names: Vec<String>,
    pub max_file_size_mb: u64,
    pub max_pdf_size_mb: u64,
    pub scan_concurrency: usize,
}

/// 扫描结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultItem {
    pub file_path: String,
    pub file_size: u64,
    pub modified_time: String,
    pub counts: HashMap<String, u32>,
    pub total: u32,
    pub unsupported_preview: bool,
}

/// 高亮区间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRange {
    pub start: usize,
    pub end: usize,
    pub type_id: String,
    pub type_name: String,
}

/// 预览结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResult {
    pub content: String,
    pub highlights: Vec<HighlightRange>,
}

/// 敏感规则定义
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveRule {
    pub id: String,
    pub name: String,
    pub regex_pattern: Option<String>,
    pub is_keyword: bool,
    pub keywords: Option<Vec<String>>,
    pub enabled_by_default: bool,
}

/// 应用配置（持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub selected_paths: Vec<String>,
    pub selected_extensions: Vec<String>,
    pub enabled_sensitive_types: Vec<String>,
    pub ignore_dir_names: Vec<String>,
    pub max_file_size_mb: u64,
    pub max_pdf_size_mb: u64,
    pub scan_concurrency: usize,
    pub theme: String,
    pub language: String,
    pub enable_experimental_parsers: bool,
    pub enable_office_parsers: bool,
    pub delete_to_trash: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            selected_paths: vec![],
            // 默认选中"*"表示所有文件类型
            selected_extensions: vec!["*".to_string()],
            enabled_sensitive_types: vec![
                "person_id".to_string(), "phone".to_string(), "email".to_string(),
                "bank_card".to_string(), "address".to_string(), "ip_address".to_string(),
                "password".to_string(),
            ],
            ignore_dir_names: vec![
                // 开发相关
                "node_modules".to_string(),
                ".git".to_string(),
                ".svn".to_string(),
                ".hg".to_string(),
                ".vscode".to_string(),
                ".idea".to_string(),
                // "target".to_string(),
                // "dist".to_string(),
                // "build".to_string(),
                
                // Windows 系统目录
                "System Volume Information".to_string(),
                // "$Recycle.Bin".to_string(),
                "Recovery".to_string(),
                "Windows".to_string(),
                "Program Files".to_string(),
                "Program Files (x86)".to_string(),
                "ProgramData".to_string(),
                "PerfLogs".to_string(),
                
                // macOS 系统目录
                ".Spotlight-V100".to_string(),
                // ".Trashes".to_string(),
                ".fseventsd".to_string(),
                ".DS_Store".to_string(),
                "Applications".to_string(),
                "Library".to_string(),
                "System".to_string(),
                
                // Linux 系统目录
                "lost+found".to_string(),
                "proc".to_string(),
                "sys".to_string(),
                "dev".to_string(),
                "run".to_string(),
                // "tmp".to_string(),
                "var".to_string(),
                "etc".to_string(),
                "bin".to_string(),
                "sbin".to_string(),
                "lib".to_string(),
                "usr".to_string(),
                "boot".to_string(),
                "mnt".to_string(),
                "media".to_string(),
                "opt".to_string(),
                "srv".to_string(),
            ],
            max_file_size_mb: 50,
            max_pdf_size_mb: 100,
            scan_concurrency: 8,
            theme: "system".to_string(),
            language: "zh-CN".to_string(),
            enable_experimental_parsers: false,
            enable_office_parsers: true,
            delete_to_trash: false, // 默认永久删除
        }
    }
}
