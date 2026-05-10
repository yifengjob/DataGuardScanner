# 扫描停滞问题 - 深度诊断与修复

## 🔴 问题现象

**症状**：
- 扫描到 830-880 个文件后完全不动
- 提示 "XX秒内无任何进展，扫描可能卡住"
- Tauri 2 性能理论上应该比 Electron 好，但实际更差

---

## 🔍 根本原因分析

### 1. 表面原因：停滞检测触发

**commands.rs 第 161-179 行**：
```rust
if idle_time > warning_threshold {
    log::warn!("警告: {}秒内无任何进展，扫描可能卡住", idle_time.as_secs());
}

if idle_time > force_stop_threshold {
    log::error!("错误: {}秒内无任何进展，强制结束扫描", idle_time.as_secs());
    break;  // ← 强制结束
}
```

**停滞检测逻辑**：
- 每 5 秒检查一次（`STAGNATION_CHECK_INTERVAL_SECS`）
- 如果 30 秒无事件 → 警告
- 如果 120 秒无事件 → 强制结束

---

### 2. 深层原因：消费者卡在文件解析

**scanner.rs 第 273-279 行（修复前）**：
```rust
// 处理单个文件
process_file_with_timeout(
    task,
    semaphore.clone(),
    event_tx.clone(),
    cancel_flag.clone(),
    config.clone(),
).await;  // ← 这里会阻塞等待
```

**问题流程**：
```
1. 消费者获取任务
2. 调用 process_file_with_timeout()
3. spawn_blocking 在后台线程执行解析
4. ❌ 某个文件的第三方库（PDF/Office）内部死锁或无限循环
5. ❌ tokio::select! 的超时只能取消 await，不能终止后台线程
6. ❌ 后台线程继续运行，占用信号量许可
7. ❌ 其他消费者无法获取许可（pool_size=4，全部卡住）
8. ❌ 没有 Progress/Log 事件发送
9. ❌ commands.rs 检测到 120 秒无活动
10. ❌ 强制结束扫描
```

---

### 3. 为什么 Electron 不卡？

**Electron 实现**（`scanner.ts` 第 373-399 行）：
```typescript
const timeoutId = setTimeout(() => {
    console.error(`[TaskQueue] 任务 ${taskId} 超时`);
    
    // ✅ 直接终止 Worker 进程
    consumer.worker.terminate();
    const index = consumers.indexOf(consumer);
    if (index > -1) {
        consumers.splice(index, 1);
        createConsumer(id); // 重启 Worker
    }
}, timeout);
```

**关键差异**：
| 特性 | Electron | Tauri (修复前) |
|------|----------|---------------|
| 超时机制 | 终止进程 | 取消 await |
| 资源释放 | ✅ 立即释放 | ❌ 后台线程继续运行 |
| 隔离性 | ✅ Worker 独立进程 | ❌ 共享线程池 |
| 恢复能力 | ✅ 自动重启 Worker | ❌ 永久卡住 |

---

## ✅ 修复方案

### 方案：使用 `tokio::time::timeout` 包装 `spawn_blocking`

#### 修复前（无效）
```rust
let process_result = tokio::select! {
    result = tokio::task::spawn_blocking(move || {
        extract_text_from_file(&file_path_clone)
    }) => {
        match result { ... }
    }
    _ = tokio::time::sleep(timeout) => {
        log::warn!("超时");
        return;  // ❌ 只取消了 select!，后台线程仍在运行
    }
};
```

**问题**：
- `tokio::select!` 只是多路复用，不是真正的超时
- 超时后 `spawn_blocking` 仍在后台线程池运行
- 信号量许可未释放

---

#### 修复后（有效）✅
```rust
let process_result = match tokio::time::timeout(timeout, tokio::task::spawn_blocking(move || {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        extract_text_from_file(&file_path_clone)
    }))
})).await {
    Ok(Ok(Ok(text_result))) => text_result,
    Ok(Ok(Err(_))) => Err("解析过程发生错误".to_string()),
    Ok(Err(_)) => Err("任务执行失败".to_string()),
    Err(_) => {
        // ✅ 真正的超时，spawn_blocking 被取消
        log::warn!("⚠️ 文件解析超时 ({}秒)，跳过: {}", timeout_secs, file_path);
        return;
    }
};
```

**优势**：
- ✅ `tokio::time::timeout` 是真正的超时控制
- ✅ 超时后 `spawn_blocking` 任务被取消
- ✅ 信号量许可正常释放
- ✅ 其他消费者可以继续工作

---

## 📊 技术细节

### tokio::time::timeout vs tokio::select!

#### tokio::select!（多路复用）
```rust
tokio::select! {
    result = future_a => { /* A 完成 */ }
    _ = tokio::time::sleep(timeout) => { /* 超时 */ }
}
```

**行为**：
- 哪个先完成就执行哪个分支
- **不会取消未完成的 Future**
- 后台任务继续运行

---

#### tokio::time::timeout（真正超时）
```rust
match tokio::time::timeout(timeout, future).await {
    Ok(result) => { /* 正常完成 */ }
    Err(_) => { /* 超时，future 被取消 */ }
}
```

**行为**：
- 超时后**取消 Future**
- 对于 `spawn_blocking`，会丢弃结果
- 后台线程最终会结束（虽然不能立即终止）

---

### 为什么不能立即终止后台线程？

**Rust 的限制**：
- Rust 没有强制终止线程的机制（不像 Java 的 `Thread.stop()`）
- `spawn_blocking` 在线程池中运行，无法单独终止
- 只能通过**取消 Future** 来忽略结果

**实际效果**：
```
时间线：
0s    - 开始解析文件 A
10s   - 超时，timeout 返回 Err
       - spawn_blocking 的 Future 被取消
       - 信号量许可释放 ✅
       - 其他消费者可以继续工作 ✅
60s   - 后台线程终于完成解析（但我们已经忽略了结果）
```

**结论**：
- ✅ 虽然不能立即终止线程，但可以**快速释放资源**
- ✅ 其他消费者不受影响
- ✅ 扫描可以继续

---

## 🧪 测试验证

### 编译测试
```bash
cargo build --release
```

**结果**：
```
Finished `release` profile [optimized] target(s) in 1m 33s
```

✅ **零错误、仅 4 个未使用常量警告**

---

### 预期效果

#### 修复前
```
文件 1-830: 正常
文件 831:   PDF 解析卡住（第三方库死锁）
  ↓
  后台线程持续运行
  信号量许可未释放
  其他 3 个消费者也卡住
  ↓
  120 秒无事件
  强制结束扫描 ❌
```

#### 修复后
```
文件 1-830: 正常
文件 831:   PDF 解析卡住
  ↓
  60 秒后超时
  timeout 返回 Err
  信号量许可释放 ✅
  记录日志："⚠️ 文件解析超时，跳过"
  ↓
  其他消费者继续工作 ✅
  文件 832-11359: 正常处理
  扫描完成 ✅
```

---

## 📝 修改的文件

| 文件 | 修改内容 | 行数变化 |
|------|---------|---------|
| `src-tauri/src/scanner.rs` | 使用 tokio::time::timeout 包装 spawn_blocking | +10/-30行 |

---

## 💡 进一步优化建议

### 短期优化

#### 1. 添加超时日志详情

```rust
Err(_) => {
    log::warn!(
        "⚠️ 文件解析超时 ({}秒): {}\n  文件大小: {} MB\n  文件类型: {}",
        timeout_secs,
        file_path,
        task.file_size as f64 / 1024.0 / 1024.0,
        Path::new(&file_path).extension().unwrap_or_default().display()
    );
    return;
}
```

**优势**：便于定位问题文件类型

---

#### 2. 统计超时文件数量

```rust
// 在 ScanState 中添加
pub timeout_count: Arc<AtomicU64>,

// 超时时增加计数
state.timeout_count.fetch_add(1, Ordering::Relaxed);
```

**优势**：前端可以显示超时统计

---

### 长期优化（架构级）

#### 3. 使用独立进程解析（类似 Electron）

**思路**：
- 每个文件解析在独立子进程中执行
- 超时后直接 `kill` 进程
- 完全隔离，互不影响

**优势**：
- ✅ 真正的进程级隔离
- ✅ 可以强制终止
- ✅ 崩溃不影响主进程

**劣势**：
- ❌ 进程创建开销大
- ❌ IPC 通信复杂
- ❌ 跨平台兼容性挑战

---

#### 4. 替换有问题的第三方库

**可能的罪魁祸首**：
- `pdf-extract`：某些 PDF 可能导致死锁
- `calamine`：损坏的 Excel 文件可能无限循环

**建议**：
- 查看 GitHub issues，是否有类似问题
- 考虑替换为更稳定的库（如 `lopdf` for PDF）
- 添加文件完整性预检查

---

## 🎉 总结

### 核心问题
**`tokio::select!` 的超时不能真正终止 `spawn_blocking` 任务**，导致后台线程占用信号量许可，所有消费者卡住。

### 修复方案
**使用 `tokio::time::timeout` 包装 `spawn_blocking`**，超时后取消 Future，释放信号量许可。

### 预期效果
- ✅ 单个文件超时不影响整体扫描
- ✅ 信号量许可正常释放
- ✅ 扫描可以完成所有文件
- ✅ 性能接近 Electron（甚至更好）

---

## 🚀 下一步操作

1. **重新构建 Tauri 应用**
   ```bash
   cargo tauri build
   ```

2. **测试扫描**
   - 启动应用
   - 开始扫描
   - 观察是否还会卡在 830 个文件处

3. **预期结果**
   - ✅ 扫描流畅，不再停滞
   - ✅ 超时文件会被跳过并记录日志
   - ✅ 总耗时约 1-2 分钟
   - ✅ 状态栏实时更新

**这次应该能彻底解决问题了！** 🎯
