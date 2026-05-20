use serde::{Deserialize, Serialize};
use crate::utils::config;
use std::collections::HashMap;

/// 目录树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct ScanConfig {
    pub selected_paths: Vec<String>,
    pub selected_extensions: Vec<String>,
    pub enabled_sensitive_types: Vec<String>,
    pub ignore_dir_names: Vec<String>,           // 忽略目录名（任意位置）
    pub system_dirs: Vec<String>,                // 系统目录完整路径
    pub max_file_size_mb: u64,
    pub max_pdf_size_mb: u64,
    pub scan_concurrency: usize,
    pub enable_builtin_rules: bool,              // 【新增】是否启用内置敏感词规则
    pub search_expression: Option<String>,       // 【新增】自定义搜索表达式
}

/// 扫描结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultItem {
    pub file_path: String,
    pub file_size: u64,
    pub modified_time: String,
    pub counts: HashMap<String, u32>,
    pub total: u32,
    pub expression_matched: Option<u32>,         // 【新增】自定义表达式匹配状态（0或1）
    pub unsupported_preview: bool,
}

/// 高亮区间
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightRange {
    pub start: usize,
    pub end: usize,
    pub type_id: String,
    pub type_name: String,
}

/// 预览结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub selected_paths: Vec<String>,
    pub selected_extensions: Vec<String>,
    pub enabled_sensitive_types: Vec<String>,
    pub ignore_dir_names: Vec<String>,           // 忽略目录名（任意位置的该名称目录都忽略）
    pub system_dirs: Vec<String>,                // 系统目录完整路径（只忽略这些特定路径）
    pub max_file_size_mb: u64,
    pub max_pdf_size_mb: u64,
    pub scan_concurrency: usize,
    pub theme: String,
    pub language: String,
    pub enable_experimental_parsers: bool,
    pub enable_office_parsers: bool,
    pub delete_to_trash: bool,
    pub ignore_other_drives_system_dirs: bool,   // 【新增】是否忽略其他磁盘的系统目录（仅 Windows）
    pub enable_builtin_rules: bool,              // 【新增】是否启用内置敏感词扫描规则
    pub search_expression: Option<String>,       // 【新增】搜索表达式（支持逻辑运算符）
}

impl Default for AppConfig {
    fn default() -> Self {
        // 【优化】使用常量定义忽略目录
        let ignore_dir_names = config::IGNORE_DIR_NAMES.iter().map(|s| s.to_string()).collect();
        
        // 【优化】使用 system_dirs 模块生成跨平台系统目录
        let system_dirs = crate::utils::system_dirs::generate_system_dirs(false);
        
        Self {
            selected_paths: vec![],
            // 默认选中"*"表示所有文件类型
            selected_extensions: vec!["*".to_string()],
            // 【优化】使用常量定义默认敏感检测类型
            enabled_sensitive_types: config::DEFAULT_SENSITIVE_TYPES.iter().map(|s| s.to_string()).collect(),
            ignore_dir_names,
            system_dirs,
            max_file_size_mb: config::DEFAULT_MAX_FILE_SIZE_MB,
            max_pdf_size_mb: config::DEFAULT_MAX_PDF_SIZE_MB,
            scan_concurrency: config::DEFAULT_CONCURRENCY_MAX,
            theme: "system".to_string(),
            language: "zh-CN".to_string(),
            enable_experimental_parsers: false,
            enable_office_parsers: true,
            delete_to_trash: false, // 默认永久删除
            ignore_other_drives_system_dirs: false, // 默认不忽略其他磁盘
            enable_builtin_rules: true, // 默认启用内置规则
            search_expression: None, // 默认无自定义表达式
        }
    }
}
