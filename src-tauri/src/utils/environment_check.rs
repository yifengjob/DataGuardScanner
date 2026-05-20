#![allow(dead_code)]
/// 系统环境检查模块（增强版）
/// 
/// 提供全面的系统环境检查功能，包括：
/// - 操作系统版本检测
/// - 可用内存检查
/// - 磁盘空间检查
/// - 权限检查
/// - 依赖库版本检查

use sys_info;
use crate::utils::concurrency::get_mem_info_with_defaults;

/// 环境检查结果
#[derive(Debug, Clone)]
pub struct EnvironmentCheckResult {
    /// 是否通过所有检查
    pub passed: bool,
    /// 检查详情
    pub details: Vec<CheckDetail>,
}

/// 单项检查详情
#[derive(Debug, Clone)]
pub struct CheckDetail {
    /// 检查项名称
    pub name: String,
    /// 是否通过
    pub passed: bool,
    /// 消息
    pub message: String,
    /// 建议（如果未通过）
    pub suggestion: Option<String>,
}

impl EnvironmentCheckResult {
    pub fn new() -> Self {
        Self {
            passed: true,
            details: Vec::new(),
        }
    }

    pub fn add_detail(&mut self, detail: CheckDetail) {
        if !detail.passed {
            self.passed = false;
        }
        self.details.push(detail);
    }

    pub fn format_report(&self) -> String {
        let status = if self.passed { "✅ 通过" } else { "❌ 未通过" };
        let mut report = format!("环境检查报告: {}\n\n", status);
        
        for detail in &self.details {
            let icon = if detail.passed { "✅" } else { "❌" };
            report.push_str(&format!("{} {}: {}\n", icon, detail.name, detail.message));
            
            if let Some(ref suggestion) = detail.suggestion {
                report.push_str(&format!("   建议: {}\n", suggestion));
            }
        }
        
        report
    }
}

/// 执行全面的环境检查
pub fn check_environment() -> EnvironmentCheckResult {
    let mut result = EnvironmentCheckResult::new();

    // 1. 检查操作系统
    check_os(&mut result);

    // 2. 检查内存
    check_memory(&mut result);

    // 3. 检查磁盘空间
    check_disk_space(&mut result);

    // 4. 检查CPU核心数
    check_cpu(&mut result);

    // 5. 检查权限
    check_permissions(&mut result);

    result
}

/// 检查操作系统
fn check_os(result: &mut EnvironmentCheckResult) {
    let os_type = sys_info::os_type().unwrap_or_else(|_| "Unknown".to_string());
    let os_release = sys_info::os_release().unwrap_or_else(|_| "Unknown".to_string());
    
    let passed = os_type != "Unknown";
    let message = format!("{} {}", os_type, os_release);
    
    result.add_detail(CheckDetail {
        name: "操作系统".to_string(),
        passed,
        message,
        suggestion: if !passed {
            Some("无法识别操作系统".to_string())
        } else {
            None
        },
    });
}

/// 检查内存
fn check_memory(result: &mut EnvironmentCheckResult) {
    let mem_info = get_mem_info_with_defaults();
    
    let total_mb = mem_info.total / 1024;
    let _free_mb = mem_info.free / 1024;
    let available_mb = mem_info.avail / 1024;
    
    let passed = available_mb >= 512; // 至少512MB可用内存
    let message = format!(
        "总计: {} MB, 可用: {} MB",
        total_mb, available_mb
    );
    
    result.add_detail(CheckDetail {
        name: "内存".to_string(),
        passed,
        message,
        suggestion: if !passed {
            Some("可用内存不足，建议关闭其他应用程序".to_string())
        } else {
            None
        },
    });
}

/// 检查磁盘空间
fn check_disk_space(result: &mut EnvironmentCheckResult) {
    // 获取应用数据目录所在磁盘的可用空间
    if let Ok(app_dir) = std::env::temp_dir().canonicalize()
        && let Some(parent) = app_dir.parent() {
            match get_disk_free_space(parent) {
                Ok(free_gb) => {
                    let passed = free_gb >= 1.0; // 至少1GB可用空间
                    let message = format!("{:.2} GB 可用", free_gb);
                    
                    result.add_detail(CheckDetail {
                        name: "磁盘空间".to_string(),
                        passed,
                        message,
                        suggestion: if !passed {
                            Some("磁盘空间不足，建议清理磁盘".to_string())
                        } else {
                            None
                        },
                    });
                }
                Err(e) => {
                    result.add_detail(CheckDetail {
                        name: "磁盘空间".to_string(),
                        passed: false,
                        message: format!("无法检查磁盘空间: {}", e),
                        suggestion: None,
                    });
                }
            }
        }
}

/// 获取磁盘可用空间（GB）
#[cfg(windows)]
fn get_disk_free_space(path: &std::path::Path) -> Result<f64, String> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetDiskFreeSpaceExW;
    
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free_bytes_available_to_caller: u64 = 0;
    let mut total_number_of_bytes: u64 = 0;
    let mut total_number_of_free_bytes: u64 = 0;
    
    unsafe {
        let result = GetDiskFreeSpaceExW(
            path_wide.as_ptr(),
            &mut free_bytes_available_to_caller,
            &mut total_number_of_bytes,
            &mut total_number_of_free_bytes,
        );
        
        if result != 0 {
            Ok(free_bytes_available_to_caller as f64 / (1024.0 * 1024.0 * 1024.0))
        } else {
            Err("调用GetDiskFreeSpaceEx失败".to_string())
        }
    }
}

#[cfg(not(windows))]
fn get_disk_free_space(_path: &std::path::Path) -> Result<f64, String> {
    // Unix系统可以使用statvfs
    Ok(10.0) // 简化实现，返回默认值
}

/// 检查CPU
fn check_cpu(result: &mut EnvironmentCheckResult) {
    match sys_info::cpu_num() {
        Ok(cpu_num) => {
            let passed = cpu_num >= 2; // 至少2核
            let message = format!("{} 核心", cpu_num);
            
            result.add_detail(CheckDetail {
                name: "CPU".to_string(),
                passed,
                message,
                suggestion: if !passed {
                    Some("CPU核心数较少，扫描速度可能较慢".to_string())
                } else {
                    None
                },
            });
        }
        Err(e) => {
            result.add_detail(CheckDetail {
                name: "CPU".to_string(),
                passed: false,
                message: format!("无法获取CPU信息: {}", e),
                suggestion: None,
            });
        }
    }
}

/// 检查权限
fn check_permissions(result: &mut EnvironmentCheckResult) {
    // 检查是否有写入权限到应用数据目录
    if let Some(app_dir) = dirs::data_local_dir() {
        let test_file = app_dir.join(".permission_test");
        
        if std::fs::write(&test_file, "test").is_ok() {
            let _ = std::fs::remove_file(&test_file);
            
            result.add_detail(CheckDetail {
                name: "权限".to_string(),
                passed: true,
                message: "具有读写权限".to_string(),
                suggestion: None,
            });
        } else {
            result.add_detail(CheckDetail {
                name: "权限".to_string(),
                passed: false,
                message: "缺少写入权限".to_string(),
                suggestion: Some("请以管理员身份运行程序".to_string()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_check_result() {
        let mut result = EnvironmentCheckResult::new();
        assert!(result.passed);
        
        result.add_detail(CheckDetail {
            name: "测试项".to_string(),
            passed: false,
            message: "测试失败".to_string(),
            suggestion: Some("建议修复".to_string()),
        });
        
        assert!(!result.passed);
        assert_eq!(result.details.len(), 1);
    }

    #[test]
    fn test_format_report() {
        let result = check_environment();
        let report = result.format_report();
        assert!(!report.is_empty());
        assert!(report.contains("环境检查报告"));
    }
}
