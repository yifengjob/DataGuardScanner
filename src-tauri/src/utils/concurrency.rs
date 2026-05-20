#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Semaphore;
use crate::utils::config;

/// 并发配置常量（已从 config 模块导入）
// use crate::config::{MEMORY_PER_WORKER_GB, CONCURRENCY_ABSOLUTE_MAX, ...};
const CONCURRENCY_MEMORY_RATIO: f64 = config::CONCURRENCY_MEMORY_RATIO;
const DEFAULT_CONCURRENCY_CPU_RATIO: f64 = config::DEFAULT_CONCURRENCY_CPU_RATIO;
const DEFAULT_CONCURRENCY_MAX: usize = config::DEFAULT_CONCURRENCY_MAX;
const DEFAULT_CONCURRENCY_MIN: usize = config::DEFAULT_CONCURRENCY_MIN;
const BYTES_TO_GB: f64 = config::BYTES_TO_GB;

/// 【新增】Worker内存使用统计
#[derive(Debug, Clone)]
pub struct WorkerMemoryStats {
    pub worker_id: usize,
    pub current_memory_mb: f64,
    pub peak_memory_mb: f64,
    pub files_processed: u64,
}

/// 【新增】全局内存管理器
pub struct MemoryManager {
    pub max_total_memory_mb: f64,
    pub max_per_worker_mb: f64,
    pub worker_stats: Vec<WorkerMemoryStats>,
}

impl MemoryManager {
    /// 创建新的内存管理器
    pub fn new(max_workers: usize) -> Self {
        // 【修复】macOS上应该使用avail而不是free
        let mem_info = get_mem_info_with_defaults();
        
        // 使用avail（available）而不是free
        let available_memory_bytes = mem_info.avail * 1024;
        let available_memory_mb = available_memory_bytes as f64 / 1024.0 / 1024.0;
        
        // 使用70%的可用内存作为总限制
        let max_total_memory_mb = available_memory_mb * 0.7;
        // 每个Worker最多使用总限制的1/max_workers
        let max_per_worker_mb = max_total_memory_mb / max_workers as f64;
        
        let worker_stats = (0..max_workers)
            .map(|id| WorkerMemoryStats {
                worker_id: id,
                current_memory_mb: 0.0,
                peak_memory_mb: 0.0,
                files_processed: 0,
            })
            .collect();
        
        log::info!(
            "[内存管理] 总内存限制: {:.1}MB, 每Worker限制: {:.1}MB, Worker数量: {}",
            max_total_memory_mb, max_per_worker_mb, max_workers
        );
        
        Self {
            max_total_memory_mb,
            max_per_worker_mb,
            worker_stats,
        }
    }
    
    /// 更新Worker内存统计
    pub fn update_worker_stats(&mut self, worker_id: usize, current_memory_mb: f64) {
        if let Some(stats) = self.worker_stats.get_mut(worker_id) {
            stats.current_memory_mb = current_memory_mb;
            if current_memory_mb > stats.peak_memory_mb {
                stats.peak_memory_mb = current_memory_mb;
            }
        }
    }
    
    /// 记录Worker处理文件数
    pub fn increment_files_processed(&mut self, worker_id: usize) {
        if let Some(stats) = self.worker_stats.get_mut(worker_id) {
            stats.files_processed += 1;
        }
    }
    
    /// 检查是否超过内存限制
    pub fn check_memory_limit(&self) -> MemoryCheckResult {
        let total_current: f64 = self.worker_stats.iter()
            .map(|s| s.current_memory_mb)
            .sum();
        
        let usage_ratio = total_current / self.max_total_memory_mb;
        
        if usage_ratio > 0.9 {
            MemoryCheckResult::Critical {
                total_mb: total_current,
                limit_mb: self.max_total_memory_mb,
                usage_percent: usage_ratio * 100.0,
            }
        } else if usage_ratio > 0.75 {
            MemoryCheckResult::Warning {
                total_mb: total_current,
                limit_mb: self.max_total_memory_mb,
                usage_percent: usage_ratio * 100.0,
            }
        } else {
            MemoryCheckResult::Normal {
                total_mb: total_current,
                limit_mb: self.max_total_memory_mb,
                usage_percent: usage_ratio * 100.0,
            }
        }
    }
    
    /// 获取内存使用报告
    pub fn get_memory_report(&self) -> String {
        let total_current: f64 = self.worker_stats.iter()
            .map(|s| s.current_memory_mb)
            .sum();
        let total_peak: f64 = self.worker_stats.iter()
            .map(|s| s.peak_memory_mb)
            .sum();
        let total_files: u64 = self.worker_stats.iter()
            .map(|s| s.files_processed)
            .sum();
        
        format!(
            "内存使用: 当前 {:.1}MB / 峰值 {:.1}MB / 限制 {:.1}MB ({:.1}%), 总处理文件: {}",
            total_current, total_peak, self.max_total_memory_mb,
            (total_current / self.max_total_memory_mb) * 100.0,
            total_files
        )
    }
}

/// 【新增】内存检查结果
#[derive(Debug, Clone)]
pub enum MemoryCheckResult {
    Normal {
        total_mb: f64,
        limit_mb: f64,
        usage_percent: f64,
    },
    Warning {
        total_mb: f64,
        limit_mb: f64,
        usage_percent: f64,
    },
    Critical {
        total_mb: f64,
        limit_mb: f64,
        usage_percent: f64,
    },
}

/// 并发数计算结果
#[derive(Debug, Clone)]
pub struct ConcurrencyInfo {
    pub actual_concurrency: usize,
    pub max_allowed_concurrency: usize,
    pub cpu_count: usize,
    pub free_memory_gb: f64,
}

/// 【新增】获取系统可用内存（GB）- 统一处理默认值和平台差异
/// 
/// macOS会将空闲内存用于文件系统缓存，free会显示很小的值
/// avail才是真正可用的内存（包括可回收的缓存）
fn get_available_memory_gb() -> f64 {
    let mem_info = get_mem_info_with_defaults();
    
    // 使用avail（available）而不是free
    let available_memory_bytes = mem_info.avail * 1024;
    available_memory_bytes as f64 / BYTES_TO_GB
}

/// 【新增】获取内存信息（带默认值）- 公共辅助函数
/// 
/// 如果sys_info失败，返回合理的默认值
/// macOS会将空闲内存用于文件系统缓存，free会显示很小的值
/// avail才是真正可用的内存（包括可回收的缓存）
pub fn get_mem_info_with_defaults() -> sys_info::MemInfo {
    sys_info::mem_info().unwrap_or(sys_info::MemInfo {
        total: (8.0 * config::BYTES_TO_GB / 1024.0) as u64, // 默认8GB总内存
        free: (2.0 * config::BYTES_TO_GB / 1024.0) as u64,  // 默认2GB空闲
        avail: (4.0 * config::BYTES_TO_GB / 1024.0) as u64, // 默认4GB可用
        buffers: 0,
        cached: 0,
        swap_total: 0,
        swap_free: 0,
    })
}

/// 根据系统硬件资源智能计算推荐的并发数
pub fn calculate_recommended_concurrency() -> ConcurrencyInfo {
    let cpu_count = num_cpus::get();
    let free_memory_gb = get_available_memory_gb();
    
    // 根据内存计算最大并发数
    let max_by_memory = (free_memory_gb * CONCURRENCY_MEMORY_RATIO / config::MEMORY_PER_WORKER_GB).floor() as usize;
    
    // 取 CPU 和内存限制的最小值，再与绝对最大值比较
    let calculated_max = cpu_count.min(max_by_memory).min(config::CONCURRENCY_ABSOLUTE_MAX);
    let max_allowed = calculated_max.max(DEFAULT_CONCURRENCY_MIN);
    
    log::info!(
        "[并发计算] CPU: {}核, 可用内存: {:.1}GB, 内存限制: {}, CPU限制: {}, 绝对最大值: {}",
        cpu_count, free_memory_gb, max_by_memory, cpu_count, config::CONCURRENCY_ABSOLUTE_MAX
    );
    
    ConcurrencyInfo {
        actual_concurrency: max_allowed,
        max_allowed_concurrency: max_allowed,
        cpu_count,
        free_memory_gb,
    }
}

/// 根据配置和系统资源计算实际使用的并发数
pub fn calculate_actual_concurrency(configured_concurrency: usize) -> ConcurrencyInfo {
    let cpu_count = num_cpus::get();
    let free_memory_gb = get_available_memory_gb();
    
    // 根据内存计算最大并发数
    let max_by_memory = (free_memory_gb * CONCURRENCY_MEMORY_RATIO / config::MEMORY_PER_WORKER_GB).floor() as usize;
    
    // 计算最大允许值
    let calculated_max = cpu_count.min(max_by_memory).min(config::CONCURRENCY_ABSOLUTE_MAX);
    let max_allowed = calculated_max.max(DEFAULT_CONCURRENCY_MIN);
    
    log::info!(
        "[并发计算] CPU: {}核, 可用内存: {:.1}GB",
        cpu_count, free_memory_gb
    );
    log::info!(
        "[并发计算] 内存限制: {}, CPU限制: {}, 绝对最大值: {}",
        max_by_memory, cpu_count, config::CONCURRENCY_ABSOLUTE_MAX
    );
    log::info!(
        "[并发计算] 计算最大值: {}, 最大允许值: {}",
        calculated_max, max_allowed
    );
    log::info!(
        "[并发计算] 配置值: {}",
        configured_concurrency
    );
    
    let actual_concurrency = if configured_concurrency > 0 {
        let result = configured_concurrency.min(max_allowed);
        log::info!(
            "[并发计算] 使用配置值: min({}, {}) = {}",
            configured_concurrency, max_allowed, result
        );
        result
    } else {
        // 自动计算：使用 CPU 核心数的比例，但不超过最大值，最少最小值
        let auto_value = (cpu_count as f64 * DEFAULT_CONCURRENCY_CPU_RATIO).floor() as usize;
        let result = auto_value.max(DEFAULT_CONCURRENCY_MIN).min(DEFAULT_CONCURRENCY_MAX);
        log::info!(
            "[并发计算] 使用自动计算: min(max(floor({} * {}), {}), {}) = {}",
            cpu_count, DEFAULT_CONCURRENCY_CPU_RATIO, DEFAULT_CONCURRENCY_MIN, DEFAULT_CONCURRENCY_MAX, result
        );
        result
    };
    
    log::info!("[并发计算] 最终并发数: {}", actual_concurrency);
    
    ConcurrencyInfo {
        actual_concurrency,
        max_allowed_concurrency: max_allowed,
        cpu_count,
        free_memory_gb,
    }
}

/// 创建信号量用于并发控制
pub fn create_semaphore(concurrency: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(concurrency))
}

/// 【新增】根据系统资源动态计算最大大文件并发数
///
/// 设计原则：
/// 1. 大文件比普通文件消耗更多内存和CPU
/// 2. 需要预留足够资源给小文件和其他系统进程
/// 3. 避免过多大文件同时解析导致GC压力过大
///
/// # Arguments
/// * `worker_count` - 当前Worker池大小
/// * `free_memory_gb` - 系统可用内存（GB）
/// * `cpu_count` - CPU核心数
///
/// # Returns
/// 推荐的大文件最大并发数
pub fn calculate_max_large_files_concurrent(
    worker_count: usize,
    free_memory_gb: f64,
    cpu_count: usize,
) -> usize {
    // ===== 1. 基于内存的限制 =====
    // 只使用一部分可用内存（留余量给小文件、系统、其他进程）
    let available_memory_for_large_files = free_memory_gb * config::LARGE_FILES_MEMORY_RATIO;
    let max_by_memory = (available_memory_for_large_files / config::MEMORY_PER_LARGE_FILE_WORKER_GB).floor() as usize;

    // ===== 2. 基于CPU的限制 =====
    // 大文件解析更耗CPU，使用更保守的比例
    let max_by_cpu = ((cpu_count as f64 * config::LARGE_FILES_CPU_RATIO).floor() as usize)
        .max(config::LARGE_FILES_CONCURRENT_MIN);

    // ===== 3. 不超过Worker总数 =====
    let max_by_workers = worker_count;

    // ===== 4. 综合计算 =====
    let calculated = max_by_memory.min(max_by_cpu).min(max_by_workers);

    // ===== 5. 应用上下限 =====
    let result = calculated
        .max(config::LARGE_FILES_CONCURRENT_MIN)
        .min(config::LARGE_FILES_CONCURRENT_ABSOLUTE_MAX);

    log::info!("[大文件并发计算]");
    log::info!(
        "  可用内存: {:.1}GB, 大文件可用: {:.1}GB",
        free_memory_gb,
        available_memory_for_large_files
    );
    log::info!(
        "  内存限制: {}, CPU限制: {}, Worker限制: {}",
        max_by_memory, max_by_cpu, max_by_workers
    );
    log::info!("  计算结果: {}, 最终值: {}", calculated, result);

    result
}
