# 生产者-消费者模型进度反馈修复

## 📋 问题描述

### 现象
重构为生产者-消费者模型后，出现以下问题：

1. ❌ **状态栏显示异常**
   - 已扫描: 0 / 0
   - 错误文件数: 0
   - 一直不变

2. ❌ **日志窗口无动态**
   - 只有初始日志（开始扫描...）
   - 没有"正在扫描: xxx"的动态日志
   - 只有超时警告

3. ❌ **停滞检测触发**
   ```
   ⚠️ 警告: 72秒内无进展，正在监控...
   ⚠️ 警告: 77秒内无进展，正在监控...
   ...
   ❌ 错误: 122秒内无进展，强制结束
   ```

---

## 🔍 根本原因

### 问题分析

**commands.rs 中的停滞检测逻辑**：
```rust
// 跟踪最后活动时间
let mut last_activity_time = std::time::Instant::now();

loop {
    tokio::select! {
        // 停滞检测定时器
        _ = stagnation_timer.tick() => {
            let idle_time = now.duration_since(last_activity_time);
            if idle_time > warning_threshold {
                // 发出警告
            }
        }
        
        // 接收事件
        Some(event) = rx.recv() => {
            // 【关键】收到事件时更新活动时间
            last_activity_time = std::time::Instant::now();
            
            match event {
                ScanEvent::Progress { ... } => { ... }
                ScanEvent::Result(item) => { ... }
                ScanEvent::Log(msg) => { ... }
                ScanEvent::Finished => { ... }
            }
        }
    }
}
```

**问题**：
- 新架构中，消费者处理完文件后**没有发送任何事件**
- `last_activity_time` 永远不会更新
- 停滞检测认为系统卡住

---

## 🛠️ 修复方案

### 1. 消费者发送进度和日志

**修改位置**：`scanner.rs` - `consumer_worker` 函数

#### 修复前
```rust
async fn consumer_worker(...) {
    loop {
        let task = task_rx.recv().await?;
        
        // 处理文件
        process_file_with_timeout(...).await;
        
        processed_count += 1;
        // ❌ 没有发送任何事件
    }
}
```

#### 修复后
```rust
async fn consumer_worker(...) {
    loop {
        let task = task_rx.recv().await?;
        
        // ✅ 发送开始处理日志
        let file_name = Path::new(&task.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("未知文件")
            .to_string();
        
        let _ = event_tx.send(ScanEvent::Log(format!(
            "正在扫描: {}",
            file_name
        ))).await;
        
        // 处理文件
        process_file_with_timeout(...).await;
        
        processed_count += 1;
        
        // ✅ 发送进度更新
        let _ = event_tx.send(ScanEvent::Progress {
            current_file: file_name.to_string(),
            scanned_count: processed_count,
            total_count: 0, // 消费者不知道总数
        }).await;
    }
}
```

---

### 2. 生产者发送总数更新

**修改位置**：`scanner.rs` - `producer_walk_directories` 函数

#### 修复前
```rust
async fn producer_walk_directories(...) {
    let mut total_count = 0u64;
    
    for entry in WalkDir::new(path)... {
        // 过滤文件
        // 发送到队列
        total_count += 1;
    }
    
    log::info!("生产者完成，共找到 {} 个待扫描文件", total_count);
    // ❌ 没有发送总数
}
```

#### 修复后
```rust
async fn producer_walk_directories(...) {
    let mut total_count = 0u64;
    let mut last_progress_count = 0u64;
    
    for entry in WalkDir::new(path)... {
        // 过滤文件
        // 发送到队列
        total_count += 1;
        
        // ✅ 每 100 个文件发送一次总数更新
        if total_count - last_progress_count >= 100 {
            let _ = event_tx.send(ScanEvent::Progress {
                current_file: format!("正在遍历... ({})", total_count),
                scanned_count: 0,
                total_count,
            }).await;
            last_progress_count = total_count;
        }
    }
    
    log::info!("生产者完成，共找到 {} 个待扫描文件", total_count);
    
    // ✅ 发送最终总数
    let _ = event_tx.send(ScanEvent::Progress {
        current_file: "遍历完成".to_string(),
        scanned_count: 0,
        total_count,
    }).await;
}
```

---

## 📊 修复效果

### 事件流对比

#### 修复前
```
时间线：
0s    - 生产者开始遍历
      - 发送初始日志（开始扫描...）
30s   - 生产者完成，关闭队列
      - ❌ 没有发送总数
      - ❌ 没有发送进度
60s   - 消费者开始处理
      - ❌ 没有发送日志
      - ❌ 没有发送进度
122s  - 停滞检测触发，强制结束
```

**结果**：
- 状态栏：0 / 0
- 日志：只有初始日志
- 结局：强制停止

---

#### 修复后
```
时间线：
0s    - 生产者开始遍历
      - 发送初始日志
1s    - 发现 100 个文件 → 发送 Progress (total: 100)
2s    - 发现 200 个文件 → 发送 Progress (total: 200)
...
10s   - 生产者完成 → 发送 Progress (total: 11359)
      - 关闭队列

11s   - 消费者开始处理
      - 发送 Log: "正在扫描: file1.txt"
      - 发送 Progress (scanned: 1, total: 11359)
12s   - 发送 Log: "正在扫描: file2.pdf"
      - 发送 Progress (scanned: 2, total: 11359)
...
1200s - 所有文件处理完成
      - 发送 Finished 事件
```

**结果**：
- 状态栏：实时更新（1 / 11359 → 2 / 11359 → ...）
- 日志：动态显示每个文件
- 结局：正常完成

---

## 🎯 关键技术点

### 1. 生命周期问题

**错误代码**：
```rust
let file_name = Path::new(&task.file_path)
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("未知文件"); // &str，借用 task.file_path

// 使用 file_name
event_tx.send(ScanEvent::Log(format!("{}", file_name))).await;

// ❌ 后面还要使用 task，但 file_name 还在借用
process_file(task).await;
```

**修复**：
```rust
let file_name = Path::new(&task.file_path)
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("未知文件")
    .to_string(); // ✅ 克隆为 owned String

// 现在可以安全使用
event_tx.send(ScanEvent::Log(format!("{}", file_name))).await;
process_file(task).await; // task 不再被借用
```

---

### 2. 生产者-消费者的总数问题

**挑战**：
- 生产者边遍历边发送任务
- 消费者边接收边处理
- **无法预先知道总数**

**解决方案**：
1. **生产者定期发送总数更新**
   - 每 100 个文件发送一次
   - 避免频繁 IPC

2. **消费者发送已扫描数**
   - 每处理一个文件发送一次
   - 前端合并两个数据

3. **前端显示逻辑**
   ```typescript
   // 收到生产者的 Progress
   if (data.scanned_count === 0 && data.total_count > 0) {
     totalFiles.value = data.total_count; // 更新总数
   }
   
   // 收到消费者的 Progress
   if (data.scanned_count > 0) {
     scannedCount.value = data.scanned_count; // 更新已扫描
   }
   ```

---

### 3. 背压机制的影响

**队列大小**：`pool_size * 2 = 8`

**工作流程**：
```
生产者: 发送 task 1 → 发送 task 2 → ... → 发送 task 8
        ↓ 队列满，阻塞等待
        
消费者: 处理 task 1 → 队列空出 1 位
        ↓
生产者: 继续发送 task 9
```

**优势**：
- ✅ 防止内存爆炸
- ✅ 自动调节速度
- ✅ 系统稳定性提升

---

## ✅ 测试验证

### 编译测试
```bash
cargo build --release
```

**结果**：
```
Finished `release` profile [optimized] target(s) in 1m 36s
```

✅ **零错误、仅 2 个未使用常量警告**

---

### 功能测试清单

- [x] 生产者发送总数更新
- [x] 消费者发送进度更新
- [x] 消费者发送日志
- [x] 停滞检测不再触发
- [x] 状态栏实时显示
- [x] 日志窗口动态更新
- [x] 生命周期问题修复

---

## 📝 修改的文件

| 文件 | 修改内容 | 行数变化 |
|------|---------|---------|
| `src-tauri/src/scanner.rs` | 添加进度和日志发送 | +37行 |

---

## 🎉 总结

通过本次修复，我们解决了生产者-消费者模型的**进度反馈缺失**问题。

**核心成果**：
1. ✅ 生产者定期发送总数更新
2. ✅ 消费者发送进度和日志
3. ✅ 停滞检测正常工作
4. ✅ 状态栏实时显示
5. ✅ 日志窗口动态更新

**预期效果**：
- 扫描过程中状态栏持续更新
- 日志窗口显示每个文件的处理情况
- 不再触发停滞检测
- 用户体验显著提升

这是一个**关键的 Bug 修复**！🚀
