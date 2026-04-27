#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod scanner;
mod file_parser;
mod sensitive_detector;
mod commands;
mod environment;

use commands::*;
use environment::check_environment;
use tauri::Manager;

fn main() {
    // 设置全局 panic hook，捕获所有未处理的 panic
    // 这对于防止 pdf-extract 等第三方库的 panic 导致程序崩溃非常重要
    std::panic::set_hook(Box::new(|info| {
        // 只记录错误信息，不打印 panic 详情
        // 这样可以避免控制台输出大量技术细节，影响用户体验
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            log::error!("⚠️ 内部错误（已自动处理）: {}", s);
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            log::error!("⚠️ 内部错误（已自动处理）: {}", s);
        } else {
            log::error!("⚠️ 发生未知内部错误（已自动处理）");
        }
        
        // 注意：不调用 default_panic，完全抑制 panic 输出
        // 因为我们的 catch_unwind 已经处理了这些错误
        // 用户只会看到友好的错误提示，不会看到技术细节
    }));
    
    // 初始化日志，过滤掉第三方库的冗余警告
    // - lopdf: 只显示 error 级别（隐藏 PDF 结构警告）
    // - pdf_extract: 只显示 error 级别（过滤掉字体 glyph 警告）
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,lopdf=error,pdf_extract=error")
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
            cancel_preview,
            open_file,
            open_file_location,
            delete_file,
            export_report,
            get_logs,
            get_sensitive_rules,
            save_config,
            load_config,
            check_system_environment,
        ])
        .setup(|app| {
            // 动态计算窗口大小
            if let Some(window) = app.get_webview_window("main") {
                // 获取主监视器
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let size = monitor.size();
                    let scale_factor = monitor.scale_factor();
                    
                    // monitor.size() 返回物理像素，需要除以 scale_factor 得到逻辑像素
                    // 然后取 80% 作为窗口大小
                    let logical_width = size.width as f64 / scale_factor;
                    let logical_height = size.height as f64 / scale_factor;
                    
                    let width = (logical_width * 0.8) as u32;
                    let height = (logical_height * 0.8) as u32;
                    
                    // 确保最小尺寸（逻辑像素）
                    let width = width.max(1000);
                    let height = height.max(600);
                    
                    log::info!("屏幕物理尺寸: {}x{}, 逻辑尺寸: {:.0}x{:.0}, 缩放比例: {}, 窗口尺寸: {}x{}", 
                               size.width, size.height, logical_width, logical_height, scale_factor, width, height);
                    
                    // 设置窗口大小（使用逻辑像素）
                    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                        width: width as f64,
                        height: height as f64,
                    }));
                    
                    // 延迟一小段时间再居中，确保窗口大小已生效
                    let window_clone = window.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        let _ = window_clone.center();
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
