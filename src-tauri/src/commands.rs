use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{State, AppHandle, Emitter};
use tokio::sync::mpsc;
use lazy_static::lazy_static;

use crate::models::*;
use crate::scanner::{run_scan_safe, ScanEvent};
use crate::file_parser::extract_text_from_file;
use crate::sensitive_detector::{get_highlights, get_builtin_rules};

lazy_static! {
    /// 预览任务取消标志（只保留最新的）
    static ref LATEST_PREVIEW_CANCEL_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
}

/// 扫描状态
pub struct ScanState {
    pub is_scanning: Arc<Mutex<bool>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub logs: Arc<Mutex<Vec<String>>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self {
            is_scanning: Arc::new(Mutex::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// 获取目录树
#[tauri::command]
pub fn get_directory_tree(path: String, show_hidden: bool) -> Result<Vec<DirectoryNode>, String> {
    let path_obj = Path::new(&path);
    
    if !path_obj.exists() {
        return Err("路径不存在".to_string());
    }
    
    let mut nodes = Vec::new();
    
    // 读取目录内容
    if let Ok(entries) = std::fs::read_dir(path_obj) {
        for entry in entries.filter_map(|e| e.ok()) {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_path = entry.path().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let is_hidden = file_name.starts_with('.');
            
            if !show_hidden && is_hidden {
                continue;
            }
            
            // 检查是否有子目录（用于懒加载）
            let has_children = is_dir && entry.path().read_dir().is_ok_and(|mut rd| rd.next().is_some());
            
            nodes.push(DirectoryNode {
                path: file_path,
                name: file_name,
                is_dir,
                is_hidden,
                has_children,
                children: None, // 懒加载，不立即加载子节点
            });
        }
    }
    
    // 按名称排序，目录在前
    nodes.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });
    
    Ok(nodes)
}

/// 开始扫描
#[tauri::command]
pub async fn scan_start(
    config: ScanConfig,
    app: AppHandle,
    state: State<'_, ScanState>,
) -> Result<(), String> {
    let mut is_scanning = state.is_scanning.lock().map_err(|e| e.to_string())?;
    if *is_scanning {
        log::warn!("扫描正在进行中，拒绝新的扫描请求");
        return Err("扫描正在进行中".to_string());
    }
    *is_scanning = true;
    drop(is_scanning);
    
    log::info!("开始新的扫描任务");
    
    // 重置取消标志
    state.cancel_flag.store(false, Ordering::Relaxed);
    state.logs.lock().map_err(|e| e.to_string())?.clear();
    
    let cancel_flag = state.cancel_flag.clone();
    
    // 创建事件通道
    let (tx, mut rx) = mpsc::channel::<ScanEvent>(100);
    
    // 启动扫描任务
    let app_clone_for_error = app.clone();
    tokio::spawn(async move {
        if let Err(e) = run_scan_safe(config, tx, cancel_flag).await {
            log::error!("扫描任务出错: {}", e);
            let _ = app_clone_for_error.emit("scan-error", e);
        }
    });
    
    // 处理事件
    let app_clone = app.clone();
    let logs_clone = state.logs.clone();
    let is_scanning_clone = state.is_scanning.clone();
    
    tokio::spawn(async move {
        let mut received_finished = false;
        
        // 设置超时，防止永远等待
        let timeout_duration = std::time::Duration::from_secs(3600); // 1小时超时
        let start_time = std::time::Instant::now();
        
        loop {
            // 检查超时
            if start_time.elapsed() > timeout_duration {
                log::error!("扫描超时");
                if let Ok(mut is_scanning) = is_scanning_clone.lock() {
                    *is_scanning = false;
                }
                let _ = app_clone.emit("scan-error", "扫描超时");
                break;
            }
            
            tokio::select! {
                Some(event) = rx.recv() => {
                    match event {
                        ScanEvent::Progress { current_file, scanned_count, total_count } => {
                            let _ = app_clone.emit("scan-progress", serde_json::json!({
                                "current_file": current_file,
                                "scanned_count": scanned_count,
                                "total_count": total_count,
                            }));
                        }
                        ScanEvent::Result(item) => {
                            let _ = app_clone.emit("scan-result", item);
                        }
                        ScanEvent::Log(msg) => {
                            logs_clone.lock().map(|mut l| l.push(msg.clone())).ok();
                            let _ = app_clone.emit("scan-log", msg);
                        }
                        ScanEvent::Finished => {
                            log::info!("扫描完成，重置状态");
                            received_finished = true;
                            let _ = app_clone.emit("scan-finished", ());
                            if let Ok(mut is_scanning) = is_scanning_clone.lock() {
                                *is_scanning = false;
                            }
                            break;
                        }
                    }
                }
                else => {
                    // 通道关闭，扫描异常结束
                    log::warn!("扫描通道关闭，强制重置状态");
                    if let Ok(mut is_scanning) = is_scanning_clone.lock() {
                        *is_scanning = false;
                    }
                    break;
                }
            }
        }
        
        if !received_finished {
            log::warn!("扫描未正常结束，已强制重置状态");
        }
    });
    
    Ok(())
}

/// 取消扫描
#[tauri::command]
pub fn scan_cancel(state: State<'_, ScanState>) -> Result<bool, String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    Ok(true)
}

/// 取消预览任务
#[tauri::command]
pub fn cancel_preview() -> Result<bool, String> {
    if let Some(flag) = LATEST_PREVIEW_CANCEL_FLAG.lock().unwrap().as_ref() {
        flag.store(true, Ordering::Relaxed);
        log::debug!("已请求取消预览任务");
        Ok(true)
    } else {
        log::warn!("没有正在进行的预览任务");
        Ok(false)
    }
}

/// 预览文件
#[tauri::command]
pub async fn preview_file(path: String, max_bytes: Option<usize>) -> Result<PreviewResult, String> {
    let max_bytes = max_bytes.unwrap_or(200 * 1024); // 默认 200KB
    
    log::debug!("开始预览任务");
    
    // 创建取消标志，并设置为最新的预览任务
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut latest_flag = LATEST_PREVIEW_CANCEL_FLAG.lock().unwrap();
        *latest_flag = Some(cancel_flag.clone());
    }
    
    // 在后台线程中执行文件读取，避免阻塞主线程
    let path_clone = path.clone();
    let cancel_flag_clone = cancel_flag.clone();
    let result = tokio::task::spawn_blocking(move || {
        // 检查是否被取消
        if cancel_flag_clone.load(Ordering::Relaxed) {
            return Err("任务已取消".to_string());
        }
        extract_text_from_file(&path_clone)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(|e| format!("文件读取失败: {}", e))?;
    
    // 再次检查是否被取消
    if cancel_flag.load(Ordering::Relaxed) {
        log::debug!("预览任务已取消（文件读取后）");
        return Err("任务已取消".to_string());
    }
    
    let (text, unsupported_preview) = result;
    
    if unsupported_preview {
        return Ok(PreviewResult {
            content: "该文件类型不支持内容预览".to_string(),
            highlights: vec![],
        });
    }
    
    // 限制预览大小（按字符边界截断，避免破坏多字节字符）
    let truncated = if text.len() > max_bytes {
        // 找到最接近 max_bytes 的字符边界
        let mut byte_idx = max_bytes;
        while byte_idx > 0 && !text.is_char_boundary(byte_idx) {
            byte_idx -= 1;
        }
        &text[..byte_idx]
    } else {
        &text
    };
    
    // 再次检查是否被取消（高亮计算前）
    if cancel_flag.load(Ordering::Relaxed) {
        log::debug!("预览任务已取消（高亮计算前）");
        return Err("任务已取消".to_string());
    }
    
    // 获取敏感规则（默认全部启用用于高亮）
    let rules = get_builtin_rules();
    let enabled_types: Vec<String> = rules.iter()
        .filter(|(_, _, enabled)| *enabled)
        .map(|(id, _, _)| id.clone())
        .collect();
    
    // 获取高亮区间
    let highlights_raw = get_highlights(truncated, &enabled_types);
    let highlights = highlights_raw.into_iter()
        .map(|(start, end, type_id, type_name)| HighlightRange {
            start,
            end,
            type_id,
            type_name,
        })
        .collect();
    
    log::debug!("预览任务完成");
    
    Ok(PreviewResult {
        content: truncated.to_string(),
        highlights,
    })
}

/// 打开文件
#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("无法打开文件: {}", e))
}

/// 删除文件（根据配置决定移入回收站或永久删除）
#[tauri::command]
pub fn delete_file(path: String) -> Result<(), String> {
    // 加载配置
    let config = load_config().map_err(|e| format!("加载配置失败: {}", e))?;
    
    if config.delete_to_trash {
        // 移入回收站
        trash::delete(&path).map_err(|e| format!("删除失败: {}", e))
    } else {
        // 永久删除
        std::fs::remove_file(&path).map_err(|e| format!("删除失败: {}", e))
    }
}

/// 导出报告
#[tauri::command]
pub fn export_report(
    results: Vec<ScanResultItem>,
    format: String,
    save_path: String,
) -> Result<String, String> {
    match format.as_str() {
        "csv" => export_csv(&results, &save_path),
        "json" => export_json(&results, &save_path),
        "xlsx" => export_xlsx(&results, &save_path),
        _ => Err("不支持的格式".to_string()),
    }
}

fn export_csv(results: &[ScanResultItem], path: &str) -> Result<String, String> {
    use std::io::Write;
    
    let mut file = std::fs::File::create(path)
        .map_err(|e| format!("无法创建文件: {}", e))?;
    
    // 写入 CSV 头
    writeln!(file, "文件路径,文件大小,修改时间,身份证数,手机号数,邮箱数,银行卡数,地址数,IP地址数,密码数,总计").ok();
    
    for item in results {
        let person_id = item.counts.get("person_id").unwrap_or(&0);
        let phone = item.counts.get("phone").unwrap_or(&0);
        let email = item.counts.get("email").unwrap_or(&0);
        let bank_card = item.counts.get("bank_card").unwrap_or(&0);
        let address = item.counts.get("address").unwrap_or(&0);
        let ip_address = item.counts.get("ip_address").unwrap_or(&0);
        let password = item.counts.get("password").unwrap_or(&0);
        
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{},{}",
            item.file_path,
            item.file_size,
            item.modified_time,
            person_id,
            phone,
            email,
            bank_card,
            address,
            ip_address,
            password,
            item.total
        ).ok();
    }
    
    Ok(path.to_string())
}

fn export_json(results: &[ScanResultItem], path: &str) -> Result<String, String> {
    let json = serde_json::to_string_pretty(results)
        .map_err(|e| format!("JSON 序列化失败: {}", e))?;
    
    std::fs::write(path, json)
        .map_err(|e| format!("写入文件失败: {}", e))?;
    
    Ok(path.to_string())
}

fn export_xlsx(results: &[ScanResultItem], path: &str) -> Result<String, String> {
    use rust_xlsxwriter::*;
    
    // 创建工作簿
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    
    // 创建样式
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::Gray)
        .set_border(FormatBorder::Thin);
    
    let number_format = Format::new()
        .set_num_format("0");
    
    let highlight_format = Format::new()
        .set_font_color(Color::Red)
        .set_bold();
    
    // 写入表头
    let headers = [
        "文件路径",
        "文件大小 (字节)",
        "修改时间",
        "身份证",
        "手机号",
        "邮箱",
        "银行卡",
        "地址",
        "IP地址",
        "密码",
        "总计",
    ];
    
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| format!("写入表头失败: {}", e))?;
    }
    
    // 设置列宽
    let _ = worksheet.set_column_width(0, 60); // 文件路径
    let _ = worksheet.set_column_width(1, 15); // 文件大小
    let _ = worksheet.set_column_width(2, 20); // 修改时间
    for col in 3..=10 {
        let _ = worksheet.set_column_width(col as u16, 10);
    }
    
    // 写入数据
    for (row_idx, item) in results.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        
        // 文件路径
        worksheet.write(row, 0, item.file_path.as_str())
            .map_err(|e| format!("写入数据失败: {}", e))?;
        
        // 文件大小
        worksheet.write_with_format(row, 1, item.file_size, &number_format)
            .map_err(|e| format!("写入数据失败: {}", e))?;
        
        // 修改时间
        worksheet.write(row, 2, item.modified_time.as_str())
            .map_err(|e| format!("写入数据失败: {}", e))?;
        
        // 敏感数据统计
        let sensitive_types = ["person_id", "phone", "email", "bank_card", "address", "ip_address", "password"];
        
        for (col_idx, type_id) in sensitive_types.iter().enumerate() {
            let count = item.counts.get(*type_id).unwrap_or(&0);
            let col = (col_idx + 3) as u16;
            
            if *count > 0 {
                worksheet.write_with_format(row, col, *count, &highlight_format)
                    .map_err(|e| format!("写入数据失败: {}", e))?;
            } else {
                worksheet.write_with_format(row, col, *count, &number_format)
                    .map_err(|e| format!("写入数据失败: {}", e))?;
            }
        }
        
        // 总计
        if item.total > 0 {
            worksheet.write_with_format(row, 10, item.total, &highlight_format)
                .map_err(|e| format!("写入数据失败: {}", e))?;
        } else {
            worksheet.write_with_format(row, 10, item.total, &number_format)
                .map_err(|e| format!("写入数据失败: {}", e))?;
        }
    }
    
    // 保存文件
    workbook.save(path)
        .map_err(|e| format!("保存 Excel 文件失败: {}", e))?;
    
    Ok(path.to_string())
}

/// 获取日志
#[tauri::command]
pub fn get_logs(state: State<'_, ScanState>) -> Result<Vec<String>, String> {
    let logs = state.logs.lock().map_err(|e| e.to_string())?;
    Ok(logs.clone())
}

/// 获取内置敏感规则
#[tauri::command]
pub fn get_sensitive_rules() -> Result<Vec<(String, String, bool)>, String> {
    Ok(get_builtin_rules())
}

/// 保存配置
#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path()?;
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化失败: {}", e))?;
    
    std::fs::write(&config_path, json)
        .map_err(|e| format!("写入配置失败: {}", e))?;
    
    Ok(())
}

/// 加载配置
#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    let config_path = get_config_path()?;
    
    if !Path::new(&config_path).exists() {
        return Ok(AppConfig::default());
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置失败: {}", e))?;
    
    let mut config: AppConfig = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置失败: {}", e))?;
    
    // 配置迁移：如果 system_dirs 为空，使用默认值
    if config.system_dirs.is_empty() {
        config.system_dirs = AppConfig::default().system_dirs;
    }
    
    Ok(config)
}

/// 获取配置文件路径
fn get_config_path() -> Result<String, String> {
    // 优先使用程序所在目录
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("获取程序路径失败: {}", e))?;
    
    let exe_dir = exe_path.parent()
        .ok_or("无法获取程序目录")?;
    
    let config_dir = exe_dir.join("data");
    
    // 如果程序目录不可写，使用用户数据目录
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).ok();
    }
    
    if (!config_dir.is_dir() || !is_writable(&config_dir))
        && let Some(user_data_dir) = dirs::data_dir() {
        let fallback_dir = user_data_dir.join("DataGuard");
        std::fs::create_dir_all(&fallback_dir).ok();
        return Ok(fallback_dir.join("config.json").to_string_lossy().to_string());
    }
    
    Ok(config_dir.join("config.json").to_string_lossy().to_string())
}

fn is_writable(path: &Path) -> bool {
    let test_file = path.join(".write_test");
    let result = std::fs::File::create(&test_file).is_ok();
    if result {
        std::fs::remove_file(&test_file).ok();
    }
    result
}

/// 检查系统环境
#[tauri::command]
pub fn check_system_environment() -> Result<crate::environment::EnvironmentCheck, String> {
    Ok(crate::environment::check_environment())
}
