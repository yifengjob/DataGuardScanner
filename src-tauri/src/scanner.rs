use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

/// 文件任务（从生产者发送到消费者）
#[derive(Debug, Clone)]
struct FileTask {
    file_path: String,
    file_size: u64,
    modified_time: String,
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

/// 执行并发扫描（生产者-消费者模型）
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
    
    // 【新增】创建共享计数器
    let scanned_count = Arc::new(AtomicU64::new(0));
    let total_count = Arc::new(AtomicU64::new(0));
    
    // 【新增】创建文件队列（生产者-消费者模型）
    let (task_tx, task_rx) = mpsc::channel::<FileTask>(pool_size * 2); // 缓冲大小为并发数的2倍
    
    // 【安全】创建信号量用于并发控制
    let semaphore = create_semaphore(pool_size);
    
    // 启动消费者池（共享同一个 Receiver）
    let task_rx = Arc::new(tokio::sync::Mutex::new(task_rx));
    
    let mut consumer_handles = Vec::new();
    for i in 0..pool_size {
        let task_rx = task_rx.clone();
        let semaphore = semaphore.clone();
        let event_tx = event_tx.clone();
        let cancel_flag = cancel_flag.clone();
        let config = config.clone();
        let scanned_count = scanned_count.clone();
        let total_count = total_count.clone();
        
        let handle = tokio::spawn(async move {
            consumer_worker(i, task_rx, semaphore, event_tx, cancel_flag, config, scanned_count, total_count).await;
        });
        
        consumer_handles.push(handle);
    }
    
    // 启动生产者（目录遍历）
    let producer_event_tx = event_tx.clone();
    let producer_total_count = total_count.clone();
    let producer_handle = tokio::spawn(async move {
        producer_walk_directories(&config, &task_tx, &cancel_flag, &producer_event_tx, producer_total_count).await;
        drop(task_tx); // 关闭发送端，通知所有消费者结束
    });
    
    // 等待生产者完成
    if let Err(e) = producer_handle.await {
        log::error!("生产者线程错误: {:?}", e);
    }
    
    // 等待所有消费者完成
    for handle in consumer_handles {
        if let Err(e) = handle.await {
            log::error!("消费者线程错误: {:?}", e);
        }
    }
    
    // 发送完成事件
    let _ = event_tx.send(ScanEvent::Finished).await;
    let _ = event_tx.send(ScanEvent::Log("扫描完成".to_string())).await;
}

/// 生产者：遍历目录并将文件任务发送到队列
async fn producer_walk_directories(
    config: &ScanConfig,
    task_tx: &mpsc::Sender<FileTask>,
    cancel_flag: &Arc<AtomicBool>,
    event_tx: &mpsc::Sender<ScanEvent>,
    total_count: Arc<AtomicU64>,
) {
    use walkdir::WalkDir;
    
    let mut local_count = 0u64;
    let mut last_progress_count = 0u64;
    
    for root_path in &config.selected_paths {
        // 【安全】检查取消标志
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = event_tx.send(ScanEvent::Log("扫描已取消".to_string())).await;
            return;
        }
        
        let path = Path::new(root_path);
        if !path.exists() || !path.is_dir() {
            continue;
        }
        
        // 遍历目录
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
            
            // 检查扩展名
            if !should_include_extension(&file_path, &config.selected_extensions) {
                continue;
            }
            
            // 检查文件大小
            if !should_include_file_by_size(&file_path, &entry, config) {
                continue;
            }
            
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
            
            // 创建文件任务
            let task = FileTask {
                file_path,
                file_size,
                modified_time,
            };
            
            // 发送到队列（如果队列满会阻塞，实现背压）
            if task_tx.send(task).await.is_err() {
                // 接收端已关闭，停止生产
                break;
            }
            
            local_count += 1;
            total_count.fetch_add(1, Ordering::Relaxed); // 更新全局总数
            
            // 【新增】每 100 个文件发送一次进度更新
            if local_count - last_progress_count >= 100 {
                let current_total = total_count.load(Ordering::Relaxed);
                let _ = event_tx.send(ScanEvent::Progress {
                    current_file: format!("正在遍历... ({})", current_total),
                    scanned_count: 0, // 生产者不处理文件
                    total_count: current_total,
                }).await;
                last_progress_count = local_count;
            }
        }
    }
    
    log::info!("生产者完成，共找到 {} 个待扫描文件", local_count);
    
    // 【新增】发送最终总数
    let final_total = total_count.load(Ordering::Relaxed);
    let _ = event_tx.send(ScanEvent::Progress {
        current_file: "遍历完成".to_string(),
        scanned_count: 0,
        total_count: final_total,
    }).await;
}

/// 消费者 Worker：从队列获取任务并处理
async fn consumer_worker(
    worker_id: usize,
    task_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<FileTask>>>,
    semaphore: Arc<tokio::sync::Semaphore>,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
    config: ScanConfig,
    scanned_count: Arc<AtomicU64>,
    total_count: Arc<AtomicU64>,
) {
    log::debug!("消费者 Worker {} 启动", worker_id);
    
    let mut processed_count = 0u64;
    
    loop {
        // 从队列获取任务
        let task = {
            let mut rx = task_rx.lock().await;
            rx.recv().await
        };
        
        // 队列关闭，退出循环
        let task = match task {
            Some(t) => t,
            None => break,
        };
        
        // 【安全】检查取消标志
        if cancel_flag.load(Ordering::Relaxed) {
            log::debug!("消费者 Worker {} 收到取消信号", worker_id);
            break;
        }
        
        // 【新增】发送开始处理日志
        let file_name = Path::new(&task.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("未知文件")
            .to_string(); // 克隆为 owned String
        let _ = event_tx.send(ScanEvent::Log(format!(
            "正在扫描: {}",
            file_name
        ))).await;
        
        // 处理单个文件
        process_file_with_timeout(
            task,
            semaphore.clone(),
            event_tx.clone(),
            cancel_flag.clone(),
            config.clone(),
        ).await;
        
        processed_count += 1;
        let current_scanned = scanned_count.fetch_add(1, Ordering::Relaxed) + 1; // 原子增加并获取新值
        let current_total = total_count.load(Ordering::Relaxed);
        
        // 【新增】发送进度更新（包含完整的 scanned 和 total）
        let _ = event_tx.send(ScanEvent::Progress {
            current_file: file_name.clone(),
            scanned_count: current_scanned,
            total_count: current_total,
        }).await;
    }
    
    log::debug!("消费者 Worker {} 完成，处理了 {} 个文件", worker_id, processed_count);
}

/// 带超时的文件处理（动态超时计算）
async fn process_file_with_timeout(
    task: FileTask,
    semaphore: Arc<tokio::sync::Semaphore>,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
    config: ScanConfig,
) {
    use crate::file_parser::extract_text_from_file;
    use crate::sensitive_detector::detect_sensitive_data;
    
    // 【新增】动态计算超时时间
    let timeout_secs = calculate_dynamic_timeout(task.file_size, &task.file_path);
    let timeout = std::time::Duration::from_secs(timeout_secs);
    
    // 【安全】获取信号量许可
    let _permit = match semaphore.acquire().await {
        Ok(permit) => permit,
        Err(e) => {
            log::error!("信号量获取失败: {}", e);
            return;
        }
    };
    
    if cancel_flag.load(Ordering::Relaxed) {
        return;
    }
    
    let file_path = task.file_path.clone();
    let file_path_clone = file_path.clone();
    
    // 【关键修复】使用 tokio::time::timeout 包装 spawn_blocking
    let process_result = match tokio::time::timeout(timeout, tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            extract_text_from_file(&file_path_clone)
        }))
    })).await {
        Ok(Ok(Ok(text_result))) => text_result,
        Ok(Ok(Err(_))) => Err("解析过程发生错误".to_string()),
        Ok(Err(_)) => Err("任务执行失败".to_string()),
        Err(_) => {
            // 超时
            log::warn!("⚠️ 文件解析超时 ({}秒)，跳过: {}", timeout_secs, file_path);
            let _ = event_tx.send(ScanEvent::Log(format!(
                "⚠️ 文件解析超时，跳过: {}", 
                Path::new(&file_path).file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("未知文件")
            ))).await;
            return;
        }
    };
    
    // 处理结果
    match process_result {
        Ok((text, unsupported_preview)) => {
            if unsupported_preview {
                return;
            }
            
            let counts = detect_sensitive_data(&text, &config.enabled_sensitive_types);
            let total: u32 = counts.values().sum();
            
            if total > 0 {
                let result = ScanResultItem {
                    file_path,
                    file_size: task.file_size,
                    modified_time: task.modified_time,
                    counts,
                    total,
                    unsupported_preview: false,
                };
                
                let _ = event_tx.send(ScanEvent::Result(result)).await;
            }
        }
        Err(e) => {
            log::debug!("解析失败 {}: {}", file_path, e);
        }
    }
}

/// 动态计算超时时间（基于文件大小和类型）
fn calculate_dynamic_timeout(file_size: u64, file_path: &str) -> u64 {
    let size_mb = file_size as f64 / config::BYTES_TO_MB as f64;
    
    // 根据文件大小分级
    let base_timeout = if size_mb < 1.0 {
        config::TIMEOUT_SMALL_FILE_SECS
    } else if size_mb < 10.0 {
        config::TIMEOUT_MEDIUM_FILE_SECS
    } else if size_mb < 50.0 {
        config::TIMEOUT_LARGE_FILE_SECS
    } else {
        config::TIMEOUT_HUGE_FILE_SECS
    };
    
    // PDF 文件需要更多时间
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    if ext == "pdf" {
        // PDF 超时增加 50%
        (base_timeout as f64 * 1.5) as u64
    } else {
        base_timeout
    }
}

/// 检查是否应该包含目录
fn should_include_directory(entry: &walkdir::DirEntry, config: &ScanConfig) -> bool {
    let name = entry.file_name().to_string_lossy();
    let path = entry.path().to_string_lossy();
    
    // 1. 检查全局忽略列表
    if config.ignore_dir_names.contains(&name.to_string()) {
        log::debug!("过滤目录（名称匹配）: {}", path);
        return false;
    }
    
    // 2. 检查系统目录
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

/// 检查是否应该包含该扩展名的文件
fn should_include_extension(file_path: &str, selected_extensions: &[String]) -> bool {
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
        true
    }
}
