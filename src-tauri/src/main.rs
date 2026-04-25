#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod models;
mod scanner;
mod file_parser;
mod sensitive_detector;
mod commands;
mod environment;

use commands::*;
use environment::check_environment;

fn main() {
    // 初始化日志，过滤掉 lopdf 库的ERROR级别日志
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,lopdf=warn")
    ).init();
    
    // 检查系统环境
    let env_check = check_environment();
    
    if !env_check.is_ready {
        // 有严重问题，显示错误信息并退出
        eprintln!("\n❌ 系统环境检查失败！\n");
        eprintln!("操作系统: {}", env_check.os_version);
        eprintln!("\n发现以下问题：\n");
        
        for (i, issue) in env_check.issues.iter().enumerate() {
            let severity_icon = match issue.severity {
                environment::IssueSeverity::Critical => "🔴",
                environment::IssueSeverity::Warning => "🟡",
            };
            
            eprintln!("{}. {} {}", i + 1, severity_icon, issue.title);
            eprintln!("   {}", issue.description);
            eprintln!("   解决方案: {}", issue.solution);
            
            if let Some(url) = &issue.download_url {
                eprintln!("   下载地址: {}", url);
            }
            eprintln!();
        }
        
        eprintln!("\n请解决上述问题后重新启动应用程序。\n");
        
        // 在 Windows 上显示图形化对话框
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            
            let message = format!(
                "系统环境检查失败！\n\n{}\n\n请查看控制台获取详细信息和下载链接。",
                env_check.issues.first()
                    .map(|i| i.title.as_str())
                    .unwrap_or("未知错误")
            );
            
            let _ = Command::new("cmd")
                .args(&["/c", "start", "cmd", "/k", "echo", &message])
                .spawn();
        }
        
        std::process::exit(1);
    }
    
    // 如果有警告，记录但不阻止启动
    if !env_check.issues.is_empty() {
        log::warn!("系统环境存在以下警告:");
        for issue in &env_check.issues {
            log::warn!("- {}: {}", issue.title, issue.description);
        }
    }
    
    log::info!("系统环境检查通过: {}", env_check.os_version);
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(ScanState::new())
        .invoke_handler(tauri::generate_handler![
            get_directory_tree,
            scan_start,
            scan_cancel,
            preview_file,
            open_file,
            delete_file,
            export_report,
            get_logs,
            get_sensitive_rules,
            save_config,
            load_config,
            check_system_environment,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
