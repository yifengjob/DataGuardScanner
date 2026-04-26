use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use walkdir::WalkDir;

use crate::models::{ScanConfig, ScanResultItem};
use crate::file_parser::extract_text_from_file;
use crate::sensitive_detector::detect_sensitive_data;

/// 扫描进度事件
#[derive(Debug, Clone)]
pub enum ScanEvent {
    Progress {
        current_file: String,
        scanned_count: u64,
        total_count: u64,
    },
    Result(ScanResultItem),
    Log(String),
    Finished,
}

/// 执行扫描（安全版本，带错误处理）
pub async fn run_scan_safe(
    config: ScanConfig,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    run_scan(config, event_tx, cancel_flag).await;
    Ok(())
}

/// 执行扫描
pub async fn run_scan(
    config: ScanConfig,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    let mut scanned_count: u64 = 0;
    let mut total_count: u64 = 0;
    
    log::info!("开始扫描，路径数: {}, 扩展名数: {}", 
        config.selected_paths.len(), 
        config.selected_extensions.len());
    
    // 发送初始日志
    event_tx.send(ScanEvent::Log("开始扫描...".to_string())).await.ok();
    event_tx.send(ScanEvent::Log(format!("扫描路径数: {}", config.selected_paths.len()))).await.ok();
    event_tx.send(ScanEvent::Log(format!("文件类型数: {}", config.selected_extensions.len()))).await.ok();
    event_tx.send(ScanEvent::Log(format!("选中的扩展名: {:?}", config.selected_extensions))).await.ok();
    event_tx.send(ScanEvent::Log(format!("敏感检测类型: {}", config.enabled_sensitive_types.join(", ")))).await.ok();
    event_tx.send(ScanEvent::Log("---".to_string())).await.ok();
    
    for root_path in &config.selected_paths {
        log::info!("扫描路径: {}", root_path);
        event_tx.send(ScanEvent::Log(format!("正在扫描: {}", root_path))).await.ok();
        
        if cancel_flag.load(Ordering::Relaxed) {
            event_tx.send(ScanEvent::Log("扫描已取消".to_string())).await.ok();
            return;
        }
        
        let path = Path::new(root_path);
        if !path.exists() {
            event_tx.send(ScanEvent::Log(format!("路径不存在: {}", root_path))).await.ok();
            continue;
        }
        
        // 检查是否是目录
        if !path.is_dir() {
            event_tx.send(ScanEvent::Log(format!("路径不是目录: {}", root_path))).await.ok();
            continue;
        }
        
        // 遍历目录
        log::debug!("开始遍历目录: {}", root_path);
        
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                should_include_directory(e, &config)
            })
        {
            if cancel_flag.load(Ordering::Relaxed) {
                event_tx.send(ScanEvent::Log("扫描已取消".to_string())).await.ok();
                return;
            }
            
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    event_tx.send(ScanEvent::Log(format!("访问错误: {}", e))).await.ok();
                    continue;
                }
            };
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            let file_path = match entry.path().to_str() {
                Some(p) => p.to_string(),
                None => continue,
            };
            
            // 检查扩展名
            if let Some(ext) = Path::new(&file_path).extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                // 如果选中了"*"，则不过滤任何文件类型
                if !config.selected_extensions.contains(&"*".to_string()) 
                    && !config.selected_extensions.contains(&ext_lower) {
                    log::debug!("跳过文件（扩展名不匹配）: {} (.{})", file_path, ext_lower);
                    continue;
                }
            } else {
                // 没有扩展名的文件，只有在选中"*"时才扫描
                if !config.selected_extensions.contains(&"*".to_string()) {
                    log::debug!("跳过文件（无扩展名）: {}", file_path);
                    continue;
                }
            }
            
            // 检查文件大小
            if let Ok(metadata) = entry.metadata() {
                let file_size = metadata.len();
                let max_size = if file_path.to_lowercase().ends_with(".pdf") {
                    config.max_pdf_size_mb * 1024 * 1024
                } else {
                    config.max_file_size_mb * 1024 * 1024
                };
                
                if file_size > max_size {
                    event_tx.send(ScanEvent::Log(format!("跳过超大文件: {} ({} MB)", 
                        file_path, file_size / 1024 / 1024))).await.ok();
                    continue;
                }
            }
            
            scanned_count += 1;
            total_count += 1;
            
            // 发送进度（每个文件都发送，确保实时更新）
            event_tx.send(ScanEvent::Progress {
                current_file: file_path.clone(),
                scanned_count,
                total_count,
            }).await.ok();
            
            // 提取文本并检测敏感数据（使用 catch_unwind 防止 panic）
            let process_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                extract_text_from_file(&file_path)
            }));
            
            match process_result {
                Ok(Ok((text, unsupported_preview))) => {
                    if unsupported_preview {
                        continue;
                    }
                    
                    let counts = detect_sensitive_data(&text, &config.enabled_sensitive_types);
                    let total: u32 = counts.values().sum();
                    
                    if total > 0 {
                        // 获取文件元数据
                        let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let modified_time = entry.path()
                            .metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .map(|t: std::time::SystemTime| {
                                let datetime: chrono::DateTime<chrono::Local> = t.into();
                                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                            })
                            .unwrap_or_else(|| "未知".to_string());
                        
                        // 记录发现的敏感文件
                        event_tx.send(ScanEvent::Log(format!(
                            "发现敏感文件: {} (总计: {} 个敏感项)",
                            file_path, total
                        ))).await.ok();
                        
                        let result = ScanResultItem {
                            file_path,
                            file_size,
                            modified_time,
                            counts,
                            total,
                            unsupported_preview: false,
                        };
                        
                        event_tx.send(ScanEvent::Result(result)).await.ok();
                    }
                }
                Ok(Err(e)) => {
                    event_tx.send(ScanEvent::Log(format!("解析失败 {}: {}", file_path, e))).await.ok();
                }
                Err(_) => {
                    event_tx.send(ScanEvent::Log(format!("文件处理时发生严重错误，跳过: {}", file_path))).await.ok();
                }
            }
        }
    }
    
    event_tx.send(ScanEvent::Finished).await.ok();
    event_tx.send(ScanEvent::Log(format!("扫描完成，共扫描 {} 个文件", scanned_count))).await.ok();
}

/// 检查是否应该包含目录（返回 true 表示保留，false 表示过滤）
fn should_include_directory(entry: &walkdir::DirEntry, config: &ScanConfig) -> bool {
    let name = entry.file_name().to_string_lossy();
    let path = entry.path().to_string_lossy();
    
    // 1. 检查全局忽略列表（精确匹配目录名，任意位置都忽略）
    if config.ignore_dir_names.contains(&name.to_string()) {
        log::debug!("过滤目录（名称匹配）: {}", path);
        return false;
    }
    
    // 2. 检查系统目录（路径前缀匹配，只在特定位置忽略）
    for system_dir in &config.system_dirs {
        if path.starts_with(system_dir) {
            log::debug!("过滤目录（系统目录）: {} (匹配: {})", path, system_dir);
            return false;
        }
    }
    
    true
}
