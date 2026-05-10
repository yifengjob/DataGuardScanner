use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::path::Path;
use tokio::sync::mpsc;

use crate::models::{ScanConfig, ScanResultItem};
use crate::concurrency::{calculate_actual_concurrency, create_semaphore};
use crate::config;

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

/// 执行并发扫描
pub async fn run_scan(
    config: ScanConfig,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    // 【优化】计算并发数并发送初始日志
    let concurrency_info = calculate_actual_concurrency(config.scan_concurrency);
    let pool_size = concurrency_info.actual_concurrency;
    
    log::info!("开始扫描，路径数: {}, 扩展名数: {}", 
        config.selected_paths.len(), 
        config.selected_extensions.len());
    log::info!("使用 {} 个并发线程 (CPU: {}核, 可用内存: {:.1}GB)", 
        pool_size, concurrency_info.cpu_count, concurrency_info.free_memory_gb);
    
    send_initial_logs(&event_tx, &config, pool_size, &concurrency_info).await;
    
    // 【安全】创建信号量用于并发控制
    let semaphore = create_semaphore(pool_size);
    
    // 【优化】收集所有需要扫描的文件
    let file_tasks = collect_file_tasks(&config, &cancel_flag, &event_tx).await;
    
    if file_tasks.is_empty() {
        log::warn!("未找到任何待扫描文件");
        event_tx.send(ScanEvent::Log("未找到任何待扫描文件".to_string())).await.ok();
        event_tx.send(ScanEvent::Finished).await.ok();
        return;
    }
    
    let total_files = file_tasks.len();
    log::info!("找到 {} 个待扫描文件", total_files);
    
    // 【优化】并发处理文件
    process_files_concurrently(
        file_tasks,
        semaphore,
        event_tx.clone(),
        cancel_flag.clone(),
        config,
    ).await;
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

/// 发送初始日志
async fn send_initial_logs(
    event_tx: &mpsc::Sender<ScanEvent>,
    config: &ScanConfig,
    pool_size: usize,
    concurrency_info: &crate::concurrency::ConcurrencyInfo,
) {
    let logs = vec![
        "开始扫描...".to_string(),
        format!("扫描路径数: {}", config.selected_paths.len()),
        format!("文件类型数: {}", config.selected_extensions.len()),
        format!("选中的扩展名: {:?}", config.selected_extensions),
        format!("敏感检测类型: {}", config.enabled_sensitive_types.join(", ")),
        format!("并发线程数: {} (CPU: {}核, 可用内存: {:.1}GB)", 
            pool_size, concurrency_info.cpu_count, concurrency_info.free_memory_gb),
        "---".to_string(),
    ];
    
    for log in logs {
        let _ = event_tx.send(ScanEvent::Log(log)).await;
    }
}

/// 收集所有需要扫描的文件
async fn collect_file_tasks(
    config: &ScanConfig,
    cancel_flag: &Arc<AtomicBool>,
    event_tx: &mpsc::Sender<ScanEvent>,
) -> Vec<(String, walkdir::DirEntry)> {
    use std::path::Path;
    use walkdir::WalkDir;
    
    let mut file_tasks = Vec::new();
    
    for root_path in &config.selected_paths {
        // 【安全】检查取消标志
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = event_tx.send(ScanEvent::Log("扫描已取消".to_string())).await;
            return file_tasks;
        }
        
        let path = Path::new(root_path);
        if !path.exists() || !path.is_dir() {
            continue;
        }
        
        // 遍历目录收集文件
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                !cancel_flag.load(Ordering::Relaxed) && should_include_directory(e, config)
            })
        {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            let file_path = match entry.path().to_str() {
                Some(p) => p.to_string(),
                None => continue,
            };
            
            // 【优化】检查扩展名
            if !should_include_extension(&file_path, &config.selected_extensions) {
                continue;
            }
            
            // 【优化】检查文件大小
            if !should_include_file_by_size(&file_path, &entry, config) {
                continue;
            }
            
            file_tasks.push((file_path, entry));
        }
    }
    
    file_tasks
}

/// 检查是否应该包含该扩展名的文件
fn should_include_extension(file_path: &str, selected_extensions: &[String]) -> bool {
    use std::path::Path;
    
    if selected_extensions.contains(&"*".to_string()) {
        return true;
    }
    
    if let Some(ext) = Path::new(file_path).extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        selected_extensions.contains(&ext_lower)
    } else {
        false
    }
}

/// 检查文件大小是否在限制范围内
fn should_include_file_by_size(
    file_path: &str,
    entry: &walkdir::DirEntry,
    config: &ScanConfig,
) -> bool {
    if let Ok(metadata) = entry.metadata() {
        let file_size = metadata.len();
        let max_size = if file_path.to_lowercase().ends_with(".pdf") {
            config.max_pdf_size_mb * config::BYTES_TO_MB
        } else {
            config.max_file_size_mb * config::BYTES_TO_MB
        };
        
        file_size <= max_size
    } else {
        true // 无法获取元数据时，默认包含
    }
}

/// 并发处理文件
async fn process_files_concurrently(
    file_tasks: Vec<(String, walkdir::DirEntry)>,
    semaphore: Arc<tokio::sync::Semaphore>,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
    config: ScanConfig,
) {
    
    let total_files = file_tasks.len();
    let mut scanned_count: u64 = 0;
    let mut completed_count: u64 = 0;
    
    // 【安全】创建任务句柄列表
    let mut join_handles = Vec::with_capacity(total_files);
    
    for (file_path, entry) in file_tasks {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }
        
        let semaphore = semaphore.clone();
        let event_tx = event_tx.clone();
        let cancel_flag = cancel_flag.clone();
        let config = config.clone();
        
        let handle = tokio::spawn(async move {
            process_single_file(
                file_path,
                entry,
                semaphore,
                event_tx,
                cancel_flag,
                config,
            ).await
        });
        
        join_handles.push(handle);
    }
    
    // 【安全】等待所有任务完成，防止资源泄漏
    for handle in join_handles {
        if let Ok(Some(_)) = handle.await {
            completed_count += 1;
        }
        scanned_count += 1;
        
        // 【优化】定期发送进度更新
        if scanned_count % config::PROGRESS_UPDATE_INTERVAL == 0 || scanned_count == total_files as u64 {
            let _ = event_tx.send(ScanEvent::Progress {
                current_file: format!("{}/{}", scanned_count, total_files),
                scanned_count,
                total_count: total_files as u64,
            }).await;
        }
    }
    
    // 发送完成事件
    let _ = event_tx.send(ScanEvent::Finished).await;
    let _ = event_tx.send(ScanEvent::Log(format!(
        "扫描完成，共扫描 {} 个文件，发现 {} 个敏感文件", 
        scanned_count, completed_count
    ))).await;
}

/// 处理单个文件
async fn process_single_file(
    file_path: String,
    entry: walkdir::DirEntry,
    semaphore: Arc<tokio::sync::Semaphore>,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
    config: ScanConfig,
) -> Option<u32> {
    use crate::file_parser::extract_text_from_file;
    use crate::sensitive_detector::detect_sensitive_data;
    
    // 【安全】获取信号量许可（控制并发）
    let _permit = semaphore.acquire().await
        .expect("信号量获取失败，可能系统资源不足");
    
    if cancel_flag.load(Ordering::Relaxed) {
        return None;
    }
    
    // 【新增】单文件解析超时保护（30秒）
    let parse_timeout = std::time::Duration::from_secs(config::SINGLE_FILE_PARSE_TIMEOUT_SECS);
    let file_path_clone = file_path.clone();
    
    let process_result = tokio::select! {
        // 正常解析流程
        result = tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                extract_text_from_file(&file_path_clone)
            }))
        }) => {
            match result {
                Ok(Ok(text_result)) => text_result,
                Ok(Err(_)) => Err("解析过程发生错误".to_string()),
                Err(_) => Err("任务执行失败".to_string()),
            }
        }
        // 超时处理
        _ = tokio::time::sleep(parse_timeout) => {
            log::warn!("⚠️ 文件解析超时 ({}秒)，跳过: {}", config::SINGLE_FILE_PARSE_TIMEOUT_SECS, file_path);
            let _ = event_tx.send(ScanEvent::Log(format!(
                "⚠️ 文件解析超时，跳过: {}", 
                Path::new(&file_path).file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("未知文件")
            ))).await;
            return None;
        }
    };
    match process_result {
        Ok((text, unsupported_preview)) => {
            if unsupported_preview {
                return None;
            }
            
            let counts = detect_sensitive_data(&text, &config.enabled_sensitive_types);
            let total: u32 = counts.values().sum();
            
            if total > 0 {
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
                
                let result = ScanResultItem {
                    file_path,
                    file_size,
                    modified_time,
                    counts,
                    total,
                    unsupported_preview: false,
                };
                
                let _ = event_tx.send(ScanEvent::Result(result)).await;
                Some(total)
            } else {
                None
            }
        }
        Err(e) => {
            // 【优化】只在 debug 模式记录错误，减少日志发送
            log::debug!("解析失败 {}: {}", file_path, e);
            None
        }
    }
}
