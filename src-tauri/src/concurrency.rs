use std::sync::Arc;
use tokio::sync::Semaphore;
use crate::config;

/// 并发配置常量（已从 config 模块导入）
// use crate::config::{MEMORY_PER_WORKER_GB, CONCURRENCY_ABSOLUTE_MAX, ...};
const CONCURRENCY_MEMORY_RATIO: f64 = config::CONCURRENCY_MEMORY_RATIO;
const DEFAULT_CONCURRENCY_CPU_RATIO: f64 = config::DEFAULT_CONCURRENCY_CPU_RATIO;
const DEFAULT_CONCURRENCY_MAX: usize = config::DEFAULT_CONCURRENCY_MAX;
const DEFAULT_CONCURRENCY_MIN: usize = config::DEFAULT_CONCURRENCY_MIN;
const BYTES_TO_GB: f64 = config::BYTES_TO_GB;

/// 并发数计算结果
#[derive(Debug, Clone)]
pub struct ConcurrencyInfo {
    pub actual_concurrency: usize,
    pub max_allowed_concurrency: usize,
    pub cpu_count: usize,
    pub free_memory_gb: f64,
}

/// 根据系统硬件资源智能计算推荐的并发数
pub fn calculate_recommended_concurrency() -> ConcurrencyInfo {
    let cpu_count = num_cpus::get();
    let free_memory_bytes = sys_info::mem_info()
        .map(|m| m.free * 1024)
        .unwrap_or((4.0 * config::BYTES_TO_GB) as u64); // 默认 4GB
    let free_memory_gb = free_memory_bytes as f64 / BYTES_TO_GB;
    
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
    let free_memory_bytes = sys_info::mem_info()
        .map(|m| m.free * 1024)
        .unwrap_or((4.0 * config::BYTES_TO_GB) as u64);
    let free_memory_gb = free_memory_bytes as f64 / BYTES_TO_GB;
    
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
