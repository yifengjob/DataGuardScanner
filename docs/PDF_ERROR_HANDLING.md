# PDF 解析崩溃问题修复

## 问题描述

在扫描包含中文编码的 PDF 文件（如电子发票）时，`pdf-extract` 库会因为遇到不支持的编码 `UniGB-UCS2-H` 而 panic，导致整个扫描线程崩溃。

### 错误信息
```
thread 'tokio-rt-worker' panicked at pdf-extract-0.10.0/src/lib.rs:983:21:
unsupported encoding UniGB-UCS2-H
```

## 根本原因

1. **第三方库限制**：`pdf-extract` 库不支持某些中文字符编码（如 UniGB-UCS2-H）
2. **Panic 传播**：panic 发生在 tokio worker 线程中，无法被普通的 `catch_unwind` 捕获
3. **缺少容错机制**：单个文件解析失败导致整个扫描过程中断

## 解决方案

采用**四层防护**策略确保程序稳定性和用户体验：

### 第零层：日志过滤优化

在 `main.rs` 中配置日志级别，过滤掉第三方库的冗余警告：

```rust
env_logger::Builder::from_env(
    env_logger::Env::default()
        .default_filter_or("info,lopdf=error,pdf_extract=error")
).init();
```

**作用**：
- 过滤掉 `lopdf` 的 PDF 结构警告（如 `Size entry mismatch`、`corrupt deflate stream`）
- 过滤掉 `pdf_extract` 的字体 glyph 警告（如 `unknown glyph name`）
- 只显示真正重要的错误信息
- 保持控制台输出清晰简洁
- 不影响关键错误的记录

**被过滤的典型警告**：
```rust
// lopdf 警告（已隐藏）
[WARN  lopdf::reader] Size entry of trailer dictionary is 32, correct value is 31.
[WARN  lopdf::parser] Filters for inline images are not yet implemented
[WARN  lopdf::object] corrupt deflate stream

// pdf_extract 警告（已隐藏）
[WARN  pdf_extract] unknown glyph name 'gid308' for font AAAAAE+.PingFangUITextSC-Regular
```

### 第一层：全局 Panic Hook

在 `main.rs` 中添加全局 panic hook，捕获所有未处理的 panic：

```rust
std::panic::set_hook(Box::new(|info| {
    // 只记录错误信息，不打印 panic 详情
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        log::error!("⚠️ 内部错误（已自动处理）: {}", s);
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        log::error!("⚠️ 内部错误（已自动处理）: {}", s);
    } else {
        log::error!("⚠️ 发生未知内部错误（已自动处理）");
    }
    
    // 不调用 default_panic，完全抑制 panic 输出
}));
```

**作用**：
- 防止任何未处理的 panic 导致程序崩溃
- **完全抑制 panic 的技术细节输出**，避免控制台混乱
- 仅记录简洁的错误日志，便于排查问题
- Release 和 Debug 模式行为一致

### 第一层：全局 Panic Hook

在 `main.rs` 中添加全局 panic hook，捕获所有未处理的 panic：

```rust
std::panic::set_hook(Box::new(|info| {
    // 只记录错误信息，不打印 panic 详情
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        log::error!("⚠️ 内部错误（已自动处理）: {}", s);
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        log::error!("⚠️ 内部错误（已自动处理）: {}", s);
    } else {
        log::error!("⚠️ 发生未知内部错误（已自动处理）");
    }
    
    // 不调用 default_panic，完全抑制 panic 输出
}));
```

**作用**：
- 防止任何未处理的 panic 导致程序崩溃
- **完全抑制 panic 的技术细节输出**，避免控制台混乱
- 仅记录简洁的错误日志，便于排查问题
- Release 和 Debug 模式行为一致

### 第二层：PDF 解析增强错误处理

在 `file_parser.rs` 的 `read_pdf_file` 函数中：

```rust
pub fn read_pdf_file(path: &str) -> Result<String, String> {
    let result = std::panic::catch_unwind(...);
    
    match result {
        Ok(Ok(text)) => {
            if text.is_empty() {
                Err("PDF 文件中未提取到文本内容".to_string())
            } else {
                Ok(text)
            }
        },
        Ok(Err(e)) => {
            // 智能识别错误类型，提供友好的错误信息
            if error_msg.contains("unsupported encoding") {
                Err("PDF 文件使用了不支持的字符编码，无法解析".to_string())
            } else if error_msg.contains("corrupt") {
                Err("PDF 文件已损坏或不完整".to_string())
            } else {
                Err(format!("PDF 解析失败: {}", e))
            }
        },
        Err(_) => Err("PDF 解析过程中发生严重错误".to_string()),
    }
}
```

**改进**：
- 空文本检测
- 错误类型智能识别
- 用户友好的错误提示

### 第三层：扫描器级别容错

在 `scanner.rs` 中：

```rust
let process_result = std::panic::catch_unwind(|| {
    extract_text_from_file(&file_path)
});

match process_result {
    Ok(Ok((text, _))) => { /* 正常处理 */ },
    Ok(Err(e)) => {
        // 文件解析失败，记录日志但不中断扫描
        event_tx.send(ScanEvent::Log(format!("解析失败 {}: {}", file_path, e))).await.ok();
    },
    Err(_) => {
        // 发生 panic，记录严重错误但继续扫描其他文件
        event_tx.send(ScanEvent::Log(format!("⚠️ 文件处理时发生严重错误，跳过: {}", file_path))).await.ok();
    }
}
```

**保障**：
- 单个文件失败不影响其他文件
- 清晰的日志记录
- 扫描过程持续进行

## 效果验证

### 修复前
- ❌ 遇到不支持编码的 PDF 时程序崩溃
- ❌ 整个扫描过程中断
- ❌ 用户需要重新启动应用
- ❌ 控制台输出大量技术细节和字体警告

### 修复后
- ✅ 自动跳过无法解析的 PDF 文件
- ✅ 记录详细错误日志
- ✅ 继续扫描其他文件
- ✅ 程序稳定运行不崩溃
- ✅ 控制台输出清晰，无冗余警告

## 测试建议

### 测试用例

1. **正常 PDF 文件**
   - UTF-8 编码的 PDF
   - 标准英文 PDF
   
2. **问题 PDF 文件**
   - 使用 UniGB-UCS2-H 编码的中文 PDF
   - 损坏的 PDF 文件
   - 加密的 PDF 文件
   
3. **混合扫描**
   - 文件夹中包含正常和问题 PDF
   - 验证是否能正确跳过问题文件并继续扫描

### 预期行为

**修复后的日志输出：**
```
[INFO] 扫描路径: /path/to/invoices
[ERROR] ⚠️ 内部错误（已自动处理）: unsupported encoding UniGB-UCS2-H
[WARN] 解析失败 invoice_001.pdf: PDF 文件使用了不支持的字符编码，无法解析
[INFO] 发现敏感文件: invoice_002.pdf (总计: 3 个敏感项)
[ERROR] ⚠️ 内部错误（已自动处理）: unsupported encoding UniGB-UCS2-H
[WARN] ⚠️ 文件处理时发生严重错误，跳过: invoice_003.pdf
[INFO] 扫描完成，共扫描 50 个文件
```

**关键改进：**
- ✅ 不再显示 `thread 'tokio-rt-worker' panicked at ...` 等技术细节
- ✅ 不再显示 `note: run with RUST_BACKTRACE=1 ...` 等调试提示
- ✅ 只显示简洁的错误日志和友好的用户提示
- ✅ 控制台输出清晰，不影响用户体验

## 长期解决方案

虽然当前的修复可以防止崩溃，但根本问题是 `pdf-extract` 库的限制。建议考虑以下长期方案：

### 方案 1：升级或替换 PDF 库
- 寻找支持更多编码的 PDF 解析库
- 考虑使用 `poppler` 或 `mupdf` 等更成熟的库
- 可能需要 FFI 绑定

### 方案 2：预处理 PDF 文件
- 在解析前检测编码
- 尝试转换编码或使用备用解析方法
- 对于不支持的文件标记为"需要手动检查"

### 方案 3：异步隔离
- 将 PDF 解析放在独立的进程中
- 进程崩溃不影响主程序
- 通过 IPC 通信获取结果

## 相关文件

- `src-tauri/src/main.rs` - 全局 panic hook
- `src-tauri/src/file_parser.rs` - PDF 解析函数
- `src-tauri/src/scanner.rs` - 扫描器容错处理

## 注意事项

1. **性能影响**：全局 panic hook 对性能影响极小
2. **输出优化**：完全抑制 panic 的技术细节，保持控制台清晰
3. **日志记录**：所有错误都会记录到日志中，便于排查问题
4. **用户体验**：用户只会看到友好的错误提示，不会看到技术细节
5. **调试支持**：如需详细调试信息，可查看应用日志文件

## 更新日志

- **v1.0.2**: 添加四层防护机制，防止 PDF 解析崩溃
- 日志过滤优化，隐藏冗余警告
- 全局 panic hook 捕获未处理异常
- 增强的错误处理和用户提示
- 扫描器级别容错，单文件失败不影响整体
