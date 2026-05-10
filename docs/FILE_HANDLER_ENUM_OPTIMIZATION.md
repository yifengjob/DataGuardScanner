# 文件处理器映射优化 - 从 if-else 链到枚举模式匹配

## 📋 问题背景

之前的 if-else 链在后期新增文件类型时存在以下问题：

### ❌ if-else 链的问题

```rust
let text = if config::TEXT_FILE_EXTENSIONS.contains(&ext.as_str()) {
    read_text_file(path)?
} else if config::PDF_EXTENSIONS.contains(&ext.as_str()) {
    read_pdf_file(path)?
} else if config::OFFICE_FILE_EXTENSIONS.contains(&ext.as_str()) {
    read_office_file(path, &ext)?
} else {
    return Err(format!("不支持的文件格式: {}", ext));
};
```

**问题分析**：
1. **扩展性差**：每新增一种文件类型，就要添加一个 `else if` 分支
2. **可读性下降**：随着类型增多，代码越来越长
3. **容易出错**：忘记添加 `else` 分支会导致编译错误或运行时 panic
4. **职责不清**：分类判断和业务逻辑混在一起

---

## ✅ 解决方案：枚举 + 模式匹配

### 设计思路

参考 Rust 的最佳实践和 Electron 项目的映射表思想，使用**枚举（Enum）+ 模式匹配（Pattern Matching）**实现更优雅的扩展方式。

---

## 🎯 实现细节

### 1. 定义文件处理器枚举

**文件**: `src-tauri/src/config.rs`

```rust
/// 文件处理器类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileHandler {
    Text,
    Pdf,
    Office,
}

impl FileHandler {
    /// 根据文件扩展名获取对应的处理器
    pub fn from_extension(ext: &str) -> Option<FileHandler> {
        if TEXT_FILE_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Text)
        } else if PDF_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Pdf)
        } else if OFFICE_FILE_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Office)
        } else {
            None
        }
    }
}
```

**优势**：
- ✅ **单一职责**：`FileHandler` 只负责分类映射
- ✅ **类型安全**：编译器确保所有情况都被处理
- ✅ **可扩展**：新增类型只需添加枚举变体

---

### 2. 业务逻辑使用模式匹配

**文件**: `src-tauri/src/file_parser.rs`

```rust
pub fn extract_text_from_file(path: &str) -> Result<(String, bool), String> {
    let ext = /* ... */;
    
    // 【优化】检查不支持预览的文件类型
    if config::UNSUPPORTED_PREVIEW_EXTENSIONS.contains(&ext.as_str()) {
        return Ok(("".to_string(), true));
    }
    
    // 【优化】使用处理器映射表，避免冗长的 if-else 链
    let handler = match config::FileHandler::from_extension(&ext) {
        Some(h) => h,
        None => return Err(format!("不支持的文件格式: {}", ext)),
    };
    
    // 根据处理器类型调用对应的解析函数
    let text = match handler {
        config::FileHandler::Text => read_text_file(path)?,
        config::FileHandler::Pdf => read_pdf_file(path)?,
        config::FileHandler::Office => read_office_file(path, &ext)?,
    };
    
    Ok((text, false))
}
```

**优势**：
- ✅ **清晰的分离**：分类判断 vs 业务逻辑
- ✅ **编译器保障**：如果新增枚举变体但未处理，编译会失败
- ✅ **易于理解**：一眼看出有哪些处理器类型

---

## 📊 扩展示例对比

### 场景：新增图片 OCR 支持

#### ❌ if-else 链方式

```rust
// 1. 在 config.rs 中添加常量
pub const IMAGE_OCR_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

// 2. 在 file_parser.rs 中修改 if-else 链
let text = if config::TEXT_FILE_EXTENSIONS.contains(&ext.as_str()) {
    read_text_file(path)?
} else if config::PDF_EXTENSIONS.contains(&ext.as_str()) {
    read_pdf_file(path)?
} else if config::OFFICE_FILE_EXTENSIONS.contains(&ext.as_str()) {
    read_office_file(path, &ext)?
} else if config::IMAGE_OCR_EXTENSIONS.contains(&ext.as_str()) {  // ← 新增
    read_image_with_ocr(path)?  // ← 新增
} else {
    return Err(format!("不支持的文件格式: {}", ext));
};
```

**问题**：
- ⚠️ 需要同时修改两个地方
- ⚠️ if-else 链越来越长
- ⚠️ 容易遗漏某个分支

---

#### ✅ 枚举方式

```rust
// 1. 在 config.rs 中添加常量和枚举变体
pub const IMAGE_OCR_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

pub enum FileHandler {
    Text,
    Pdf,
    Office,
    ImageOcr,  // ← 新增
}

impl FileHandler {
    pub fn from_extension(ext: &str) -> Option<FileHandler> {
        if TEXT_FILE_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Text)
        } else if PDF_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Pdf)
        } else if OFFICE_FILE_EXTENSIONS.contains(&ext) {
            Some(FileHandler::Office)
        } else if IMAGE_OCR_EXTENSIONS.contains(&ext) {  // ← 新增
            Some(FileHandler::ImageOcr)  // ← 新增
        } else {
            None
        }
    }
}

// 2. 在 file_parser.rs 中添加 match 分支
let text = match handler {
    config::FileHandler::Text => read_text_file(path)?,
    config::FileHandler::Pdf => read_pdf_file(path)?,
    config::FileHandler::Office => read_office_file(path, &ext)?,
    config::FileHandler::ImageOcr => read_image_with_ocr(path)?,  // ← 新增
};
```

**优势**：
- ✅ **编译器辅助**：如果忘记添加 `ImageOcr` 分支，编译会报错
- ✅ **结构清晰**：每个变体的处理逻辑独立
- ✅ **易于维护**：新增类型有明确的步骤

---

## 🔧 进一步优化：使用 HashMap 映射表

如果需要更高的灵活性，可以使用 **HashMap** 实现真正的映射表：

```rust
use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref FILE_HANDLER_MAP: HashMap<&'static str, FileHandler> = {
        let mut map = HashMap::new();
        
        // 文本文件
        for ext in TEXT_FILE_EXTENSIONS {
            map.insert(*ext, FileHandler::Text);
        }
        
        // PDF 文件
        for ext in PDF_EXTENSIONS {
            map.insert(*ext, FileHandler::Pdf);
        }
        
        // Office 文件
        for ext in OFFICE_FILE_EXTENSIONS {
            map.insert(*ext, FileHandler::Office);
        }
        
        map
    };
}

impl FileHandler {
    pub fn from_extension(ext: &str) -> Option<FileHandler> {
        FILE_HANDLER_MAP.get(ext).copied()
    }
}
```

**优势**：
- ✅ **O(1) 查找**：HashMap 查找性能优于多个 `contains()`
- ✅ **配置驱动**：可以轻松从配置文件加载映射关系
- ✅ **动态更新**：运行时可以动态添加新的映射

**缺点**：
- ⚠️ 需要额外的依赖（`lazy_static`）
- ⚠️ 初始化开销（虽然只在首次使用时）
- ⚠️ 对于小数据集（<50个元素），性能提升不明显

---

## 📈 方案对比总结

| 特性 | if-else 链 | 枚举 + match | HashMap 映射表 |
|------|-----------|-------------|---------------|
| **可扩展性** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **类型安全** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **编译器检查** | ❌ | ✅ | ❌ |
| **性能** | O(n) | O(n) | O(1) |
| **代码清晰度** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **维护成本** | 高 | 低 | 中 |
| **适用场景** | <3种类型 | 3-10种类型 | >10种类型 |

---

## 💡 最佳实践建议

### 1. 当前项目选择：**枚举 + match** ✅

**原因**：
- 文件类型数量适中（3类）
- 需要编译器保障类型安全
- 代码清晰易维护
- 性能足够好

### 2. 何时切换到 HashMap？

当满足以下条件时：
- 文件类型超过 10 种
- 需要从配置文件动态加载
- 需要运行时动态添加新类型
- 性能成为瓶颈（需要 profiling 确认）

### 3. 未来扩展模板

```rust
// 新增文件类型的标准流程：

// Step 1: 在 config.rs 中添加扩展名常量
pub const NEW_TYPE_EXTENSIONS: &[&str] = &["ext1", "ext2"];

// Step 2: 在 FileHandler 枚举中添加变体
pub enum FileHandler {
    // ...
    NewType,
}

// Step 3: 在 from_extension 中添加映射
if NEW_TYPE_EXTENSIONS.contains(&ext) {
    Some(FileHandler::NewType)
}

// Step 4: 在 file_parser.rs 的 match 中添加处理
config::FileHandler::NewType => read_new_type_file(path)?,

// Step 5: 实现解析函数
fn read_new_type_file(path: &str) -> Result<String, String> {
    // ...
}
```

---

## 🎉 总结

通过引入 **枚举 + 模式匹配**，我们实现了：

1. ✅ **消除冗长的 if-else 链**
2. ✅ **编译器保障类型安全**
3. ✅ **清晰的扩展流程**
4. ✅ **更好的可维护性**
5. ✅ **符合 Rust 最佳实践**

这是一个**生产级别**的代码质量改进！🚀
