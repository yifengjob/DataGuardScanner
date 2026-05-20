use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::path::Path;
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

use crate::models::{ScanConfig, ScanResultItem};
use crate::utils::concurrency::{calculate_actual_concurrency, calculate_max_large_files_concurrent, create_semaphore};
use crate::utils::config;
use crate::utils::scanner_helpers::{StagnationDetector, StagnationStatus};
use crate::utils::power_manager::PowerManager;
use crate::core::scheduler::MultiQueueScheduler;
// 【注意】日志宏通过 #[macro_use] 自动导入，无需显式 use

/// 扫描进度事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanEvent {
    Progress {
        current_file: String,
        scanned_count: u64,
        total_count: u64,
        // 【新增】过滤和跳过计数
        #[serde(skip_serializing_if = "Option::is_none")]
        filtered_count: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skipped_count: Option<u64>,
    },
    /// 【优化】批量结果（用于减少IPC调用）
    BatchResult(Vec<ScanResultItem>),
    Log(String),
    Finished,
}

/// 文件任务（从生产者发送到消费者）
#[derive(Debug, Clone)]
pub struct FileTask {
    pub file_path: String,
    pub file_size: u64,
    pub modified_time: String,
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

/// 执行并发扫描（生产者-消费者模型 + 智能调度）
pub async fn run_scan(
    config: ScanConfig,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    // 【优化】计算并发数并发送初始日志
    let concurrency_info = calculate_actual_concurrency(config.scan_concurrency);
    let pool_size = concurrency_info.actual_concurrency;
    
    log_info!("开始扫描，路径数: {}, 扩展名数: {}", 
        config.selected_paths.len(), 
        config.selected_extensions.len());
    log_info!("使用 {} 个并发线程 (CPU: {}核, 可用内存: {:.1}GB)", 
        pool_size, concurrency_info.cpu_count, concurrency_info.free_memory_gb);
    
    send_initial_logs(&event_tx, &config, pool_size, &concurrency_info).await;
    
    // 【电源管理】启动电源阻止，防止扫描过程中系统休眠
    let mut power_manager = PowerManager::new();
    if let Err(e) = power_manager.start(true, true) {
        log_warn!("⚠️ 电源阻止启动失败: {}，扫描将继续但可能被中断", e);
    }
    
    // 【新增】创建共享计数器
    let scanned_count = Arc::new(AtomicU64::new(0));
    let total_count = Arc::new(AtomicU64::new(0));
    
    // 【智能调度】动态计算大文件并发数
    let max_large_concurrent = calculate_max_large_files_concurrent(
        pool_size,
        concurrency_info.free_memory_gb,
        concurrency_info.cpu_count,
    );
    let scheduler = Arc::new(MultiQueueScheduler::new(max_large_concurrent));
    log_info!("[智能调度] 启用多队列调度，大文件并发限制: {} (动态计算)", max_large_concurrent);
    
    // 【P0修复】生产者完成标志 - 用于通知消费者可以安全退出
    let producer_done = Arc::new(AtomicBool::new(false));
    
    // 【安全】创建信号量用于并发控制
    let semaphore = create_semaphore(pool_size);
    
    // 【智能调度】启动消费者池 - 使用scheduler获取任务
    let mut consumer_handles = Vec::new();
    for i in 0..pool_size {
        let sem = semaphore.clone();
        let etx = event_tx.clone();
        let cf = cancel_flag.clone();
        let cfg = config.clone();
        let sc = scanned_count.clone();
        let tc = total_count.clone();
        let sched = scheduler.clone();
        let pd = producer_done.clone();
        
        let handle = tokio::spawn(async move {
            smart_consumer_worker(
                i,
                sched,
                pd,
                sem,
                etx,
                cf,
                cfg,
                sc,
                tc,
            ).await;
        });
        consumer_handles.push(handle);
    }
    
    // 启动生产者（目录遍历 + 智能调度入队）
    let producer_event_tx = event_tx.clone();
    let producer_total_count = total_count.clone();
    let producer_scheduler = scheduler.clone();
    let producer_done_flag = producer_done.clone();
    let producer_handle = tokio::spawn(async move {
        producer_with_smart_enqueue(&config, &producer_scheduler, &cancel_flag, &producer_event_tx, producer_total_count).await;
        // 【P0修复】标记生产者完成
        producer_done_flag.store(true, Ordering::SeqCst);
        log_debug!("[智能调度] 生产者已完成，通知所有消费者");
        // 【关键修复】唤醒所有等待的消费者
        producer_scheduler.notify_all();
    });
    
    // 【关键修复】不等待生产者完成，直接等待消费者
    // 让消费者和生产者并发执行
    // if let Err(e) = producer_handle.await {
    //     log_error!("生产者线程错误: {:?}", e);
    // }
    
    // 等待所有消费者完成
    for handle in consumer_handles {
        if let Err(e) = handle.await {
            log_error!("消费者线程错误: {:?}", e);
        }
    }
    
    // 【新增】等待生产者完成（消费者已经完成，现在可以安全等待生产者）
    if let Err(e) = producer_handle.await {
        log_error!("生产者线程错误: {:?}", e);
    }
    
    // 【电源管理】停止电源阻止，恢复系统正常休眠
    if let Err(e) = power_manager.stop() {
        log_warn!("⚠️ 电源阻止停止失败: {}", e);
    }
    
    // 发送完成事件
    let _ = event_tx.send(ScanEvent::Finished).await;
    let _ = event_tx.send(ScanEvent::Log("扫描完成".to_string())).await;
}

/// 带超时的文件处理（动态超时计算 + 流式处理）
async fn process_file_with_timeout(
    task: FileTask,
    semaphore: Arc<tokio::sync::Semaphore>,
    cancel_flag: Arc<AtomicBool>,
    config: ScanConfig,
) -> Option<ScanResultItem> {
    use crate::core::file_parser::extract_text_streaming;
    
    // 【新增】动态计算超时时间
    let timeout_secs = calculate_dynamic_timeout(task.file_size, &task.file_path);
    let timeout = std::time::Duration::from_secs(timeout_secs);
    
    // 【安全】获取信号量许可
    let _permit = match semaphore.acquire().await {
        Ok(permit) => permit,
        Err(e) => {
            log_error!("信号量获取失败: {}", e);
            return None;
        }
    };
    
    if cancel_flag.load(Ordering::Relaxed) {
        return None;
    }
    
    let file_path = task.file_path.clone();
    let file_path_for_async = file_path.clone(); // 克隆用于async闭包
    let enabled_types = config.enabled_sensitive_types.clone();
    
    // 【关键修改】使用流式处理替代一次性读取
    let process_result = tokio::time::timeout(timeout, async move {
        extract_text_streaming(&file_path_for_async, &enabled_types).await
    }).await;
    
    match process_result {
        Ok(Ok(stats)) => {
            // 流式处理已完成敏感数据检测
            log_debug!("文件 {} 检测结果: sensitive_count={}", file_path, stats.sensitive_count);
            if stats.sensitive_count > 0 {
                log_info!("✅ 发现敏感文件: {} ({} 处敏感内容)", file_path, stats.sensitive_count);
                Some(ScanResultItem {
                    file_path,
                    file_size: task.file_size,
                    modified_time: task.modified_time,
                    counts: stats.counts.clone(),  // ✅ 使用详细计数
                    total: stats.sensitive_count as u32,
                    expression_matched: stats.expression_matched,  // ✅ 使用表达式匹配状态
                    unsupported_preview: false,
                })
            } else {
                None
            }
        }
        Ok(Err(e)) => {
            log_debug!("流式处理失败 {}: {}", file_path, e);
            None
        }
        Err(_) => {
            log_warn!("⚠️ 文件处理超时 ({}秒)，跳过: {}", timeout_secs, file_path);
            None
        }
    }
}

/// 动态计算超时时间（基于文件大小和类型）
fn calculate_dynamic_timeout(file_size: u64, file_path: &str) -> u64 {
    let size_mb = file_size as f64 / config::BYTES_TO_MB as f64;
    
    // 【优化】使用非线性公式计算基础超时
    // 公式: base_timeout = min_timeout + (max_timeout - min_timeout) * (1 - e^(-size_mb/k))
    // 其中 k 是衰减系数，控制增长速度
    
    // 计算基础超时（指数增长曲线）
    let base_timeout = config::DYNAMIC_TIMEOUT_MIN_SECS 
        + (config::DYNAMIC_TIMEOUT_MAX_SECS - config::DYNAMIC_TIMEOUT_MIN_SECS) 
        * (1.0 - (-size_mb / config::DYNAMIC_TIMEOUT_DECAY_K).exp());
    
    // 【优化】根据文件类型调整超时
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // 不同文件类型的超时倍数
    let type_multiplier = match ext.as_str() {
        "pdf" => config::TIMEOUT_MULTIPLIER_PDF,
        "docx" | "doc" | "wps" => config::TIMEOUT_MULTIPLIER_WORD,
        "xlsx" | "xls" | "et" => config::TIMEOUT_MULTIPLIER_EXCEL,
        "pptx" | "ppt" | "dps" => config::TIMEOUT_MULTIPLIER_POWERPOINT,
        _ => config::TIMEOUT_MULTIPLIER_DEFAULT,
    };
    
    // 计算最终超时时间
    let final_timeout = (base_timeout * type_multiplier) as u64;
    
    // 确保在合理范围内
    final_timeout.clamp(
        config::DYNAMIC_TIMEOUT_MIN_SECS as u64, 
        config::DYNAMIC_TIMEOUT_MAX_SECS as u64
    )
}

/// 发送初始日志
async fn send_initial_logs(
    event_tx: &mpsc::Sender<ScanEvent>,
    config: &ScanConfig,
    pool_size: usize,
    concurrency_info: &crate::utils::concurrency::ConcurrencyInfo,
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

// ==================== 【智能调度】新增函数 ====================

/// 智能消费者 Worker：从 scheduler 获取任务并处理
async fn smart_consumer_worker(
    worker_id: usize,
    scheduler: Arc<MultiQueueScheduler>,
    producer_done: Arc<AtomicBool>,
    semaphore: Arc<tokio::sync::Semaphore>,
    event_tx: mpsc::Sender<ScanEvent>,
    cancel_flag: Arc<AtomicBool>,
    config: ScanConfig,
    scanned_count: Arc<AtomicU64>,
    total_count: Arc<AtomicU64>,
) {
    // 【降级】改为DEBUG，减少启动时的日志噪音
    log_debug!("🚀 智能消费者 Worker {} 启动", worker_id);
    
    // 【新增】批量结果缓冲区
    let mut result_buffer = Vec::with_capacity(config::RESULT_BATCH_SIZE);
    let mut last_batch_time = std::time::Instant::now();
    
    // 【新增】停滞检测器
    let mut stagnation_detector = StagnationDetector::new(
        config::STAGNATION_WARNING_THRESHOLD_SECS,
        config::STAGNATION_FORCE_STOP_THRESHOLD_SECS,
        config::STAGNATION_CHECK_INTERVAL_SECS,
    );
    
    // 【新增】自适应进度节流状态
    let mut progress_count = 0u64;
    
    let mut processed_count = 0u64;
    
    loop {
        // 【智能调度】从 scheduler 获取任务
        let task = scheduler.select_optimal_task();
        
        // 【删除】过度频繁的日志，严重影响性能
        // log::info!("🔍 Worker {} 检查任务: {:?}", worker_id, task.is_some());
        
        // 如果队列为空，检查是否应该退出
        let task = match task {
            Some(t) => t,
            None => {
                // 【降级】改为DEBUG，减少日志噪音
                log_debug!("⚪ Worker {} 队列为空，检查生产者状态", worker_id);
                // 【P0修复】检查生产者是否已完成
                if producer_done.load(Ordering::SeqCst) {
                    log_debug!("⚠️ Worker {} 生产者已完成，检查队列长度", worker_id);
                    // 生产者已完成，再次检查队列是否为空
                    if scheduler.get_queue_length() == 0 {
                        log_info!("❌ Worker {} 检测到生产者完成且队列为空，退出", worker_id);
                        break;
                    }
                }
                
                // 检查取消标志
                if cancel_flag.load(Ordering::Relaxed) {
                    log_info!("❌ Worker {} 收到取消信号", worker_id);
                    break;
                }
                
                // 【降级】改为DEBUG
                log_debug!("⏸️ Worker {} 等待新任务通知...", worker_id);
                // 【P1优化】使用Notify替代忙等待 - 等待新任务通知
                scheduler.get_notify().notified().await;
                log_debug!("✅ Worker {} 被唤醒，重新检查队列", worker_id);
                continue;
            }
        };
        
        // 【安全】检查取消标志
        if cancel_flag.load(Ordering::Relaxed) {
            log_debug!("智能消费者 Worker {} 收到取消信号", worker_id);
            break;
        }
        
        // 【智能调度】标记任务开始处理
        scheduler.mark_task_started(&task);
        
        // 【删除】每个文件都发送“正在扫描”日志会造成大量IPC调用，严重影响性能
        // 文件扫描进度应该通过Progress事件更新，而不是Log事件
        // let file_name = Path::new(&task.file_path)
        //     .file_name()
        //     .and_then(|n| n.to_str())
        //     .unwrap_or("未知文件")
        //     .to_string();
        // let _ = event_tx.send(ScanEvent::Log(format!(
        //     "正在扫描: {}",
        //     file_name
        // ))).await;
        
        // 提取文件名用于进度更新
        let file_name = Path::new(&task.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("未知文件")
            .to_string();
        
        // 处理单个文件
        if let Some(result) = process_file_with_timeout(
            task.clone(),
            semaphore.clone(),
            cancel_flag.clone(),
            config.clone(),
        ).await {
            // 添加到批量缓冲区
            result_buffer.push(result);
            
            // 检查是否应该发送批量结果
            let should_flush = result_buffer.len() >= config::RESULT_BATCH_SIZE
                || last_batch_time.elapsed().as_millis() >= config::RESULT_BATCH_TIMEOUT_MS as u128;
            
            if should_flush && !result_buffer.is_empty() {
                // 【优化】批量发送（一次性发送所有结果）
                let batch: Vec<ScanResultItem> = std::mem::take(&mut result_buffer);
                let _ = event_tx.send(ScanEvent::BatchResult(batch)).await;
                last_batch_time = std::time::Instant::now();
            }
        }
        
        // 【智能调度】标记任务完成
        scheduler.mark_task_completed(&task);
        
        processed_count += 1;
        let current_scanned = scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
        let current_total = total_count.load(Ordering::Relaxed);
        
        // 【新增】标记活动（用于停滞检测）
        stagnation_detector.mark_activity();
        
        // 【新增】检查停滞状态
        match stagnation_detector.check_stagnation() {
            StagnationStatus::Warning(secs) => {
                let _ = event_tx.send(ScanEvent::Log(format!(
                    "⚠️ 扫描可能停滞，已 {} 秒无进展",
                    secs
                ))).await;
            }
            StagnationStatus::Critical(secs) => {
                log_error!("🛑 扫描严重停滞，已 {} 秒无进展，强制停止", secs);
                let _ = event_tx.send(ScanEvent::Log(format!(
                    "🛑 扫描严重停滞，已 {} 秒无进展",
                    secs
                ))).await;
                cancel_flag.store(true, Ordering::Relaxed);
                break;
            }
            StagnationStatus::Normal => {}
        }
        
        // 【新增】自适应进度更新策略
        let should_update_progress = if current_scanned <= config::PROGRESS_INITIAL_FAST_COUNT {
            true
        } else if current_total >= config::PROGRESS_MASSIVE_FILE_THRESHOLD {
            progress_count.is_multiple_of(config::PROGRESS_MASSIVE_UPDATE_INTERVAL)
        } else {
            progress_count.is_multiple_of(config::PROGRESS_UPDATE_INTERVAL)
        };
        
        if should_update_progress {
            let _ = event_tx.send(ScanEvent::Progress {
                current_file: file_name.clone(),
                scanned_count: current_scanned,
                total_count: current_total,
                filtered_count: None, // 消费者阶段不统计过滤和跳过
                skipped_count: None,
            }).await;
        }
        
        progress_count += 1;
    }
    
    // 【优化】发送剩余的批量结果
    if !result_buffer.is_empty() {
        let batch: Vec<ScanResultItem> = std::mem::take(&mut result_buffer);
        let _ = event_tx.send(ScanEvent::BatchResult(batch)).await;
    }
    
    log_debug!("智能消费者 Worker {} 结束，共处理 {} 个文件", worker_id, processed_count);
}

/// 智能生产者：遍历目录并使用 scheduler 入队（复用producer模块）
async fn producer_with_smart_enqueue(
    config: &ScanConfig,
    scheduler: &Arc<MultiQueueScheduler>,
    cancel_flag: &Arc<AtomicBool>,
    event_tx: &mpsc::Sender<ScanEvent>,
    total_count: Arc<AtomicU64>,
) {
    use crate::core::producer::producer_walk_directories_core;
    
    // 复用核心逻辑，只改变任务分发方式
    producer_walk_directories_core(
        config,
        cancel_flag,
        event_tx,
        total_count,
        |task| {
            scheduler.enqueue_task(task);
        },
    ).await;
}
