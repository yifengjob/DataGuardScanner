/// 智能任务调度器（Electron版对齐）
/// 
/// 设计思想：
/// 1. 按文件类型和大小分类到不同队列
/// 2. 大文件有独立的并发限制（防止资源竞争和OOM）
/// 3. Worker不空闲原则：除非达到大文件限制或没有任务
/// 
/// 四层调度策略：
/// - 策略1: 大文件优先保障（队列有大文件且未达并发上限）
/// - 策略2: 选择不同类型的小文件（允许同类型并行）
/// - 策略3: 类型超时检查（防止死锁）
/// - 策略4: 兜底方案（违反类型互斥但遵守大文件限制）

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::Notify;

use crate::core::scanner::FileTask;
use crate::utils::config;

/// 按类型和大小的队列结构
#[derive(Debug)]
pub struct TypeQueues {
    /// 大文件队列（>= LARGE_FILE_THRESHOLD_MB）
    pub large: VecDeque<FileTask>,
    /// 小文件队列（< LARGE_FILE_THRESHOLD_MB）
    pub small: VecDeque<FileTask>,
}

impl TypeQueues {
    pub fn new() -> Self {
        Self {
            large: VecDeque::new(),
            small: VecDeque::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.large.is_empty() && self.small.is_empty()
    }

    pub fn total_count(&self) -> usize {
        self.large.len() + self.small.len()
    }
}

/// 调度器状态跟踪
pub struct SchedulerState {
    /// 正在处理的文件类型计数
    pub processing_type_count: Mutex<HashMap<String, usize>>,
    /// 正在处理的大文件数量
    pub large_files_processing: AtomicUsize,
    /// 每种类型最后被调度的时间（用于超时检测）
    pub last_type_schedule_time: Mutex<HashMap<String, Instant>>,
    /// 下一个要处理的类型索引（轮询策略）
    pub next_type_index: AtomicUsize,
    /// 【P1优化】通知机制 - 替代忙等待
    pub notify: Notify,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self {
            processing_type_count: Mutex::new(HashMap::new()),
            large_files_processing: AtomicUsize::new(0),
            last_type_schedule_time: Mutex::new(HashMap::new()),
            next_type_index: AtomicUsize::new(0),
            notify: Notify::new(),
        }
    }
}

/// 多队列调度器
pub struct MultiQueueScheduler {
    /// 按文件类型和大小的队列
    queue_by_type: Arc<Mutex<HashMap<String, TypeQueues>>>,
    /// 调度器状态
    state: Arc<SchedulerState>,
    /// 大文件并发限制
    max_large_concurrent: usize,
}

impl MultiQueueScheduler {
    /// 创建新的调度器
    pub fn new(max_large_concurrent: usize) -> Self {
        Self {
            queue_by_type: Arc::new(Mutex::new(HashMap::new())),
            state: Arc::new(SchedulerState::new()),
            max_large_concurrent,
        }
    }

    /// 向队列中添加任务
    pub fn enqueue_task(&self, task: FileTask) {
        let file_type = self.get_file_type(&task);
        let is_large = self.is_large_file(&task);
        
        let mut queues = self.queue_by_type.lock().unwrap();
        
        // 确保该类型的队列存在
        if !queues.contains_key(&file_type) {
            queues.insert(file_type.clone(), TypeQueues::new());
        }
        
        let type_queues = queues.get_mut(&file_type).unwrap();
        
        if is_large {
            type_queues.large.push_back(task);
        } else {
            type_queues.small.push_back(task);
        }
        
        // 【P1优化】通知等待的consumer有新任务
        self.state.notify.notify_one();
    }

    /// 从队列中移除并返回一个任务
    pub fn dequeue_task(&self, file_type: &str, is_large: bool) -> Option<FileTask> {
        let mut queues = self.queue_by_type.lock().unwrap();
        
        let type_queues = queues.get_mut(file_type)?;
        
        let task = if is_large {
            type_queues.large.pop_front()
        } else {
            type_queues.small.pop_front()
        };
        
        // 清理空队列
        if type_queues.is_empty() {
            queues.remove(file_type);
        }
        
        task
    }

    /// 获取队列中的总任务数
    pub fn get_queue_length(&self) -> usize {
        let queues = self.queue_by_type.lock().unwrap();
        queues.values().map(|q| q.total_count()).sum()
    }

    /// 检查是否还有大文件在队列中
    #[allow(dead_code)]
    pub fn has_large_files_in_queue(&self) -> bool {
        let queues = self.queue_by_type.lock().unwrap();
        queues.values().any(|q| !q.large.is_empty())
    }

    /// 智能选择最优任务（四层策略）
    pub fn select_optimal_task(&self) -> Option<FileTask> {
        // 【P1优化】一次性获取所有需要的信息，避免多次加锁
        let (type_order, has_large) = {
            let queues = self.queue_by_type.lock().unwrap();
            let type_order: Vec<String> = queues.keys().cloned().collect();
            let has_large = queues.values().any(|q| !q.large.is_empty());
            (type_order, has_large)
        };

        if type_order.is_empty() {
            return None;
        }

        // ==================== 策略1: 大文件优先保障 ====================
        // 目标：确保在大文件处理完之前，始终保持 max_large_concurrent 个Worker在处理大文件
        // 条件：队列中有大文件 + 当前处理的大文件数 < 上限
        let current_large = self.state.large_files_processing.load(Ordering::SeqCst);
        
        if has_large && current_large < self.max_large_concurrent {
            let next_idx = self.state.next_type_index.load(Ordering::SeqCst);
            
            for i in 0..type_order.len() {
                let idx = (next_idx + i) % type_order.len();
                let file_type = &type_order[idx];
                
                if !self.is_type_blocked(file_type, true)
                    && let Some(task) = self.dequeue_task(file_type, true) {
                        // 更新轮询索引
                        self.state.next_type_index.store(
                            (idx + 1) % type_order.len(), 
                            Ordering::SeqCst
                        );
                        
                        log::debug!("[智能调度] 策略1: 大文件优先保障 {}", file_type);
                        return Some(task);
                    }
            }
        }

        // ==================== 策略2: 选择不同类型的小文件 ====================
        // 目标：确保Worker不闲置，同时提高吞吐量
        let next_idx = self.state.next_type_index.load(Ordering::SeqCst);
        
        for i in 0..type_order.len() {
            let idx = (next_idx + i) % type_order.len();
            let file_type = &type_order[idx];
            
            // 优先大文件（如果未达上限且类型未被阻塞）
            if current_large < self.max_large_concurrent {
                if !self.is_type_blocked(file_type, true)
                    && let Some(task) = self.dequeue_task(file_type, true) {
                        self.state.next_type_index.store(
                            (idx + 1) % type_order.len(),
                            Ordering::SeqCst
                        );
                        log::debug!("[智能调度] 策略2: 选择大文件 {}", file_type);
                        return Some(task);
                    }
            }
            
            // 其次选择小文件（不检查类型阻塞，允许同类型并行）✅
            if let Some(task) = self.dequeue_task(file_type, false) {
                self.state.next_type_index.store(
                    (idx + 1) % type_order.len(),
                    Ordering::SeqCst
                );
                log::debug!("[智能调度] 策略2: 选择小文件 {}（允许同类型并行）", file_type);
                return Some(task);
            }
        }

        // ==================== 策略3: 类型超时检查 ====================
        // 目标：防止死锁（当所有类型都被阻塞时）
        if let Some(task) = self.check_type_timeout_and_select(&type_order) {
            log::debug!("[智能调度] 策略3: 类型超时，允许同类型");
            return Some(task);
        }

        // ==================== 策略4: 兜底 - 违反类型互斥，但遵守大文件限制 ====================
        // 目标：确保Worker不闲置（宁可违反类型互斥）
        // 原则：允许多个Worker处理同类型，但不违反大文件规则
        
        // 优先选择大文件（如果未达上限）
        if current_large < self.max_large_concurrent {
            for file_type in &type_order {
                if let Some(task) = self.dequeue_task(file_type, true) {
                    log::debug!("[智能调度] 策略4: 所有类型阻塞，选择大文件 {}（违反类型互斥）", file_type);
                    return Some(task);
                }
            }
        }
        
        // 其次选择小文件（即使违反类型互斥）
        for file_type in &type_order {
            if let Some(task) = self.dequeue_task(file_type, false) {
                log::debug!("[智能调度] 策略4: 所有类型阻塞，选择小文件 {}（违反类型互斥）", file_type);
                return Some(task);
            }
        }

        // 唯一能让Worker闲置的情况：全是大文件且已达上限
        log::debug!("[智能调度] 策略4: 全是大文件且已达上限，Worker等待中...");
        None
    }

    /// 检查类型是否被阻塞（同类型已达上限）
    fn is_type_blocked(&self, file_type: &str, is_large: bool) -> bool {
        let count = {
            let counts = self.state.processing_type_count.lock().unwrap();
            *counts.get(file_type).unwrap_or(&0)
        };
        
        if is_large {
            // 大文件：严格互斥，最多 1 个并发
            count >= 1
        } else {
            // 小文件：不阻塞，允许同类型并行
            false
        }
    }

    /// 检查类型超时，如果超时则允许同类型
    fn check_type_timeout_and_select(&self, type_order: &[String]) -> Option<FileTask> {
        let now = Instant::now();
        let timeout_ms = config::TYPE_MUTEX_TIMEOUT_MS;
        
        let mut last_times = self.state.last_type_schedule_time.lock().unwrap();
        
        for file_type in type_order {
            // 获取最后调度时间，如果不存在则视为“已超时”（可以立即调度）
            let last_time = last_times.get(file_type).copied();
            let should_allow = match last_time {
                Some(t) => {
                    let elapsed = now.duration_since(t).as_millis();
                    elapsed > timeout_ms as u128
                }
                None => true, // 从未调度过，视为已超时
            };
            
            if should_allow {
                log::info!(
                    "[智能调度] 类型 '{}' {}，解除互斥",
                    file_type,
                    if last_time.is_some() {
                        format!("超时 {:.1}s", now.duration_since(last_time.unwrap()).as_secs_f64())
                    } else {
                        "首次调度".to_string()
                    }
                );
                
                // 直接从队列中获取（优先小文件）
                if let Some(task) = self.dequeue_task(file_type, false) {
                    // 更新调度时间
                    last_times.insert(file_type.clone(), Instant::now());
                    return Some(task);
                }
                
                if let Some(task) = self.dequeue_task(file_type, true) {
                    // 更新调度时间
                    last_times.insert(file_type.clone(), Instant::now());
                    return Some(task);
                }
            }
        }
        
        None
    }

    /// 标记任务开始处理（更新状态）
    pub fn mark_task_started(&self, task: &FileTask) {
        let file_type = self.get_file_type(task);
        let is_large = self.is_large_file(task);
        
        // 更新类型计数
        {
            let mut counts = self.state.processing_type_count.lock().unwrap();
            let count = counts.entry(file_type.clone()).or_insert(0);
            *count += 1;
        }
        
        // 更新大文件计数
        if is_large {
            self.state.large_files_processing.fetch_add(1, Ordering::SeqCst);
        }
        
        // 更新最后调度时间
        {
            let mut times = self.state.last_type_schedule_time.lock().unwrap();
            times.insert(file_type, Instant::now());
        }
    }

    /// 标记任务完成（释放状态）
    pub fn mark_task_completed(&self, task: &FileTask) {
        let file_type = self.get_file_type(task);
        let is_large = self.is_large_file(task);
        
        // 更新类型计数
        {
            let mut counts = self.state.processing_type_count.lock().unwrap();
            if let Some(count) = counts.get_mut(&file_type) {
                if *count > 0 {
                    *count -= 1;
                } else {
                    // 【防御性编程】计数异常，记录日志
                    log::warn!(
                        "[智能调度] 类型 '{}' 的计数异常（当前为0，尝试减1）",
                        file_type
                    );
                }
            } else {
                // 【防御性编程】类型不存在于计数中
                log::warn!(
                    "[智能调度] 类型 '{}' 不在处理计数中，可能未调用mark_task_started",
                    file_type
                );
            }
        }
        
        // 更新大文件计数
        if is_large {
            let previous = self.state.large_files_processing.fetch_sub(1, Ordering::SeqCst);
            if previous == 0 {
                log::warn!("[智能调度] 大文件计数异常（当前为0，尝试减1）");
            }
        }
    }

    /// 获取文件类型
    fn get_file_type(&self, task: &FileTask) -> String {
        Path::new(&task.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
    }

    /// 判断是否为大文件
    fn is_large_file(&self, task: &FileTask) -> bool {
        let file_size_mb = task.file_size as f64 / config::BYTES_TO_MB as f64;
        file_size_mb >= config::SCHEDULER_LARGE_FILE_THRESHOLD_MB
    }

    /// 获取当前大文件并发数
    #[allow(dead_code)]
    pub fn get_current_large_files(&self) -> usize {
        self.state.large_files_processing.load(Ordering::SeqCst)
    }

    /// 【P1优化】获取通知器引用 - 用于consumer等待新任务
    pub fn get_notify(&self) -> &Notify {
        &self.state.notify
    }

    /// 【关键修复】唤醒所有等待的consumer - 生产者完成时调用
    pub fn notify_all(&self) {
        // Notify::notify_waiters() 会唤醒所有等待者
        self.state.notify.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::scanner::FileTask;

    fn create_test_task(file_path: &str, file_size_mb: u64) -> FileTask {
        FileTask {
            file_path: file_path.to_string(),
            file_size: file_size_mb * config::BYTES_TO_MB,
            modified_time: "2024-01-01 00:00:00".to_string(),
        }
    }

    #[test]
    fn test_enqueue_and_dequeue() {
        let scheduler = MultiQueueScheduler::new(2);
        
        // 添加小文件
        let task1 = create_test_task("test.txt", 1);
        scheduler.enqueue_task(task1.clone());
        
        assert_eq!(scheduler.get_queue_length(), 1);
        
        // 取出任务
        let dequeued = scheduler.dequeue_task("txt", false);
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().file_path, "test.txt");
        
        assert_eq!(scheduler.get_queue_length(), 0);
    }

    #[test]
    fn test_large_file_classification() {
        let scheduler = MultiQueueScheduler::new(2);
        
        // 50MB的文件应该是大文件（阈值是50MB）
        let large_task = create_test_task("large.pdf", 50);
        scheduler.enqueue_task(large_task);
        
        // 10MB的文件应该是小文件
        let small_task = create_test_task("small.txt", 10);
        scheduler.enqueue_task(small_task);
        
        assert!(scheduler.has_large_files_in_queue());
        assert_eq!(scheduler.get_queue_length(), 2);
    }

    #[test]
    fn test_strategy1_large_file_priority() {
        let scheduler = MultiQueueScheduler::new(2);
        
        // 添加大文件和小文件
        scheduler.enqueue_task(create_test_task("large1.pdf", 100));
        scheduler.enqueue_task(create_test_task("small1.txt", 1));
        scheduler.enqueue_task(create_test_task("small2.txt", 1));
        
        // 策略1应该优先选择大文件
        let task = scheduler.select_optimal_task();
        assert!(task.is_some());
        assert!(task.as_ref().unwrap().file_path.contains("large"));
        
        // 标记任务开始，模拟正在处理
        scheduler.mark_task_started(task.as_ref().unwrap());
        assert_eq!(scheduler.get_current_large_files(), 1);
    }

    #[test]
    fn test_strategy2_small_file_parallel() {
        let scheduler = MultiQueueScheduler::new(2);
        
        // 添加多个同类型小文件
        scheduler.enqueue_task(create_test_task("file1.txt", 1));
        scheduler.enqueue_task(create_test_task("file2.txt", 1));
        scheduler.enqueue_task(create_test_task("file3.txt", 1));
        
        // 策略2应该允许同类型小文件并行
        let task1 = scheduler.select_optimal_task();
        let task2 = scheduler.select_optimal_task();
        
        assert!(task1.is_some());
        assert!(task2.is_some());
        assert!(task1.as_ref().unwrap().file_path.contains(".txt"));
        assert!(task2.as_ref().unwrap().file_path.contains(".txt"));
    }

    #[test]
    fn test_large_file_concurrency_limit() {
        let scheduler = MultiQueueScheduler::new(2);
        
        // 添加3个大文件
        scheduler.enqueue_task(create_test_task("large1.pdf", 100));
        scheduler.enqueue_task(create_test_task("large2.pdf", 100));
        scheduler.enqueue_task(create_test_task("large3.pdf", 100));
        
        // 分配2个大文件（达到上限）
        let task1 = scheduler.select_optimal_task();
        let task2 = scheduler.select_optimal_task();
        
        scheduler.mark_task_started(task1.as_ref().unwrap());
        scheduler.mark_task_started(task2.as_ref().unwrap());
        
        assert_eq!(scheduler.get_current_large_files(), 2);
        
        // 第3个大文件应该无法立即分配（需要等待前两个完成）
        // 但由于没有其他类型的文件，策略4会违反类型互斥
        let task3 = scheduler.select_optimal_task();
        // 在策略4中，如果全是大文件且已达上限，返回None
        assert!(task3.is_none());
        
        // 完成一个大文件
        scheduler.mark_task_completed(task1.as_ref().unwrap());
        assert_eq!(scheduler.get_current_large_files(), 1);
        
        // 现在可以分配第3个大文件
        let task3 = scheduler.select_optimal_task();
        assert!(task3.is_some());
    }

    #[test]
    fn test_type_timeout_mechanism() {
        let scheduler = MultiQueueScheduler::new(2);
        
        // 添加一个类型的文件
        scheduler.enqueue_task(create_test_task("file1.pdf", 1));
        
        // 第一次调度
        let task1 = scheduler.select_optimal_task();
        assert!(task1.is_some());
        scheduler.mark_task_started(task1.as_ref().unwrap());
        
        // 添加同类型的另一个文件
        scheduler.enqueue_task(create_test_task("file2.pdf", 1));
        
        // 由于大文件严格互斥，第二个文件应该无法立即分配
        // （但在这个测试中，因为是小文件，所以会被策略2分配）
        // 我们需要测试大文件的超时机制
    }
}
