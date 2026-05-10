/// 系统目录配置管理
/// 根据操作系统平台生成默认的系统目录列表

use std::env;
use crate::config;

/// 获取基础系统目录（不包含其他磁盘）
pub fn get_base_system_dirs() -> Vec<String> {
    let platform = env::consts::OS;
    
    match platform {
        "windows" => config::WINDOWS_SYSTEM_DIRS_C_DRIVE.iter().map(|s| s.to_string()).collect(),
        "macos" => config::MACOS_SYSTEM_DIRS.iter().map(|s| s.to_string()).collect(),
        "linux" => config::LINUX_SYSTEM_DIRS.iter().map(|s| s.to_string()).collect(),
        _ => vec![], // 未知平台返回空列表
    }
}

/// 根据配置生成完整的系统目录列表
/// 
/// # Arguments
/// * `ignore_other_drives` - 是否忽略其他磁盘的系统目录（仅 Windows 有效）
/// 
/// # Returns
/// 完整的系统目录列表
pub fn generate_system_dirs(ignore_other_drives: bool) -> Vec<String> {
    let base_dirs = get_base_system_dirs();
    
    // 仅在 Windows 且启用选项时添加其他磁盘
    if env::consts::OS == "windows" && ignore_other_drives {
        let mut all_dirs = base_dirs.clone();
        
        // 添加 D-Z 盘的系统目录
        for drive_char in b'D'..=b'Z' {
            let drive = drive_char as char;
            for template in config::WINDOWS_OTHER_DRIVES_SYSTEM_DIRS {
                all_dirs.push(template.replace("{}", &drive.to_string()));
            }
        }
        
        all_dirs
    } else {
        base_dirs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_base_system_dirs_windows() {
        if env::consts::OS == "windows" {
            let dirs = get_base_system_dirs();
            assert!(dirs.contains(&"C:\\Windows".to_string()));
            assert!(dirs.contains(&"C:\\Program Files".to_string()));
        }
    }

    #[test]
    fn test_get_base_system_dirs_macos() {
        if env::consts::OS == "macos" {
            let dirs = get_base_system_dirs();
            assert!(dirs.contains(&"/System".to_string()));
            assert!(dirs.contains(&"/Applications".to_string()));
        }
    }

    #[test]
    fn test_get_base_system_dirs_linux() {
        if env::consts::OS == "linux" {
            let dirs = get_base_system_dirs();
            assert!(dirs.contains(&"/proc".to_string()));
            assert!(dirs.contains(&"/usr".to_string()));
        }
    }

    #[test]
    fn test_generate_system_dirs_no_other_drives() {
        let dirs = generate_system_dirs(false);
        // 不忽略其他磁盘时，应该只有基础目录
        if env::consts::OS == "windows" {
            assert!(!dirs.is_empty());
            // 不应该包含 D 盘
            assert!(!dirs.iter().any(|d| d.starts_with("D:\\")));
        }
    }

    #[test]
    fn test_generate_system_dirs_ignore_other_drives() {
        let dirs = generate_system_dirs(true);
        // Windows 下忽略其他磁盘时，应该包含 D-Z 盘
        if env::consts::OS == "windows" {
            assert!(dirs.iter().any(|d| d.starts_with("D:\\Windows")));
            assert!(dirs.iter().any(|d| d.starts_with("Z:\\Program Files")));
        }
    }
}
