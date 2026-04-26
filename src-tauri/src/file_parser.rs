use std::fs;
use std::path::Path;
use encoding_rs::GBK;

/// 读取文本文件内容，自动检测编码
pub fn read_text_file(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("无法读取文件: {}", e))?;
    
    // 尝试 UTF-8 解码
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        return Ok(text);
    }
    
    // 回退到 GBK
    let (text, _, had_errors) = GBK.decode(&bytes);
    if had_errors {
        return Err("文件编码无法识别".to_string());
    }
    
    Ok(text.into_owned())
}

/// 读取 PDF 文件并提取文本
pub fn read_pdf_file(path: &str) -> Result<String, String> {
    use pdf_extract::extract_text;
    
    // 使用 catch_unwind 捕获可能的 panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        extract_text(path)
    }));
    
    match result {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(format!("PDF 解析失败: {}", e)),
        Err(_) => Err("PDF 解析过程中发生错误（可能是损坏的文件）".to_string()),
    }
}

/// 读取 Office 文档（.docx, .xlsx, .pptx, .doc, .ppt, .wps, .et）
pub fn read_office_file(path: &str, ext: &str) -> Result<String, String> {
    match ext {
        "xlsx" | "xls" | "et" => {
            // Excel 表格（包括 WPS 表格）
            read_excel_file(path)
        }
        "docx" | "pptx" => {
            // 简化：对于 docx/pptx，尝试作为 zip 读取部分文本
            read_docx_pptx_simple(path)
        }
        "doc" | "wps" => {
            // 旧版 Word 文档（包括 WPS 文字）
            read_doc_file(path)
        }
        "ppt" => {
            // 旧版 PowerPoint 演示文稿
            read_ppt_file(path)
        }
        _ => Err(format!("不支持的 Office 格式: {}", ext)),
    }
}

/// 读取 Excel 文件
fn read_excel_file(path: &str) -> Result<String, String> {
    use calamine::{open_workbook_auto, Reader};
    
    // 使用 catch_unwind 捕获可能的 panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| format!("Excel 解析失败: {}", e))?;
        
        let mut text = String::new();
        
        // 遍历所有工作表
        for sheet in workbook.sheet_names().to_owned() {
            if let Ok(range) = workbook.worksheet_range(&sheet) {
                for row in range.rows() {
                    let cells: Vec<String> = row.iter()
                        .map(|cell| cell.to_string())
                        .collect();
                    text.push_str(&cells.join("\t"));
                    text.push('\n');
                }
                text.push_str("---\n");
            }
        }
        
        Ok(text)
    }));
    
    match result {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Excel 文件解析过程中发生错误（可能是损坏的文件）".to_string()),
    }
}

/// 简单读取 docx/pptx（仅提取基本文本）
fn read_docx_pptx_simple(path: &str) -> Result<String, String> {
    use std::io::Read;
    
    // 使用 catch_unwind 捕获可能的 panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let file = fs::File::open(path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("ZIP 解析失败: {}", e))?;
        
        let mut text = String::new();
        
        // 尝试提取主要文本内容 - 先检查 docx
        let has_doc = archive.by_name("word/document.xml").is_ok();
        
        if has_doc {
            let mut file = archive.by_name("word/document.xml")
                .map_err(|e| format!("无法读取文档: {}", e))?;
            let mut content = String::new();
            file.read_to_string(&mut content).ok();
            // 简单去除 XML 标签
            text = strip_xml_tags(&content);
        } else {
            // 尝试 pptx
            if let Ok(mut file) = archive.by_name("ppt/slides/slide1.xml") {
                let mut content = String::new();
                file.read_to_string(&mut content).ok();
                text = strip_xml_tags(&content);
            }
        }
        
        if text.is_empty() {
            return Err("无法提取文本内容".to_string());
        }
        
        Ok(text)
    }));
    
    match result {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Office 文档解析过程中发生错误（可能是损坏的文件）".to_string()),
    }
}

/// 读取旧版 .doc 文件（简化版）
fn read_doc_file(path: &str) -> Result<String, String> {
    // 尝试从文件中提取可打印文本
    // 这是一个简化的方法：查找连续的 ASCII/UTF-8 文本块
    let file = fs::read(path)
        .map_err(|e| format!("无法读取文件: {}", e))?;
    
    let text = extract_text_from_binary(&file);
    
    if text.is_empty() {
        return Err("无法从 .doc 文件中提取文本".to_string());
    }
    
    Ok(text)
}

/// 读取旧版 .ppt 文件（简化版）
fn read_ppt_file(path: &str) -> Result<String, String> {
    // 尝试从文件中提取可打印文本
    let file = fs::read(path)
        .map_err(|e| format!("无法读取文件: {}", e))?;
    
    let text = extract_text_from_binary(&file);
    
    if text.is_empty() {
        return Err("无法从 .ppt 文件中提取文本".to_string());
    }
    
    Ok(text)
}

/// 从二进制数据中提取可打印文本
fn extract_text_from_binary(data: &[u8]) -> String {
    let mut result = String::new();
    let mut current_text = String::new();
    let min_text_length = 4; // 最少连续字符数
    
    for &byte in data {
        // 检查是否是可打印字符（ASCII 32-126 或常见中文字符范围）
        if (32..=126).contains(&byte) || byte == b'\n' || byte == b'\r' || byte == b'\t' {
            current_text.push(byte as char);
        } else {
            // 非可打印字符，检查累积的文本是否足够长
            if current_text.len() >= min_text_length {
                // 清理空白字符
                let cleaned = current_text.trim();
                if !cleaned.is_empty() {
                    result.push_str(cleaned);
                    result.push('\n');
                }
            }
            current_text.clear();
        }
    }
    
    // 处理最后的文本块
    if current_text.len() >= min_text_length {
        let cleaned = current_text.trim();
        if !cleaned.is_empty() {
            result.push_str(cleaned);
        }
    }
    
    // 过滤掉太短的行和纯数字行
    let lines: Vec<&str> = result.lines()
        .filter(|line| line.len() > 2)
        .collect();
    
    lines.join("\n")
}

/// 去除 XML 标签
fn strip_xml_tags(xml: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    
    for ch in xml.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    
    result
}

/// 根据扩展名选择合适的解析器
pub fn extract_text_from_file(path: &str) -> Result<(String, bool), String> {
    let path_obj = Path::new(path);
    let ext = path_obj.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    let unsupported_preview = matches!(ext.as_str(), "dps" | "zip" | "rar" | "7z" | "tar" | "gz");
    
    if unsupported_preview {
        return Ok(("".to_string(), true));
    }
    
    let text = match ext.as_str() {
        "txt" | "log" | "md" | "ini" | "conf" | "cfg" | "env" |
        "js" | "ts" | "py" | "java" | "c" | "cpp" | "go" | "rs" |
        "php" | "rb" | "swift" |
        "csv" | "json" | "xml" | "yaml" | "yml" | "properties" | "toml" => {
            read_text_file(path)?
        }
        "pdf" => {
            read_pdf_file(path)?
        }
        "xlsx" | "xls" | "docx" | "pptx" | "doc" | "ppt" | "wps" | "et" => {
            read_office_file(path, &ext)?
        }
        _ => {
            return Err(format!("不支持的文件格式: {}", ext));
        }
    };
    
    Ok((text, false))
}
