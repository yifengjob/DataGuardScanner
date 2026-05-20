/// 流式文件处理器
/// 
/// 实现滑动窗口重叠策略，支持大文件的流式处理
/// 内存峰值控制在 ~5MB（CHUNK_SIZE + OVERLAP）

use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};


use crate::core::sensitive_detector::detect_sensitive_data;
use crate::utils::config;

/// 分块大小：5MB（从 config 导入）
const CHUNK_SIZE: usize = config::STREAM_CHUNK_SIZE;

/// 重叠区大小：200字符（从 config 导入）
const OVERLAP_SIZE: usize = config::STREAM_OVERLAP_SIZE;

/// 流式处理器配置
#[allow(dead_code)]
pub struct StreamProcessorConfig {
    /// 启用的敏感数据类型（规则ID列表）
    pub enabled_types: Vec<String>,
    /// 是否仅预览模式（不检测敏感数据）
    pub preview_mode: bool,
    /// 【新增】是否启用内置规则
    pub enable_builtin_rules: bool,
    /// 【新增】自定义搜索表达式
    pub search_expression: Option<String>,
}

/// 处理结果统计
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProcessStats {
    /// 处理的总字节数
    pub total_bytes: u64,
    /// 处理的总字符数
    pub total_chars: usize,
    /// 处理的块数
    pub chunks_processed: usize,
    /// 发现的敏感数据数量
    pub sensitive_count: usize,
    /// 【新增】详细分类计数（规则ID -> 匹配次数）
    pub counts: std::collections::HashMap<String, u32>,
    /// 【新增】自定义表达式匹配状态（0或1）
    pub expression_matched: Option<u32>,
}

/// 流式文件处理器
#[allow(dead_code)]
pub struct FileStreamProcessor {
    /// 累积缓冲区
    buffer: String,
    /// 上一块的重叠尾部
    previous_overlap: String,
    /// 已处理的总字节数
    total_bytes: u64,
    /// 已处理的总字符数（用于高亮偏移计算）
    total_chars: usize,
    /// 当前块索引
    chunk_index: usize,
    /// 全局行偏移
    global_line_offset: usize,
    /// 处理统计
    stats: ProcessStats,
}

/// 【通用宏】生成流式处理函数
/// 用于消除重复的Rc<RefCell>模式代码
macro_rules! impl_streaming_processor {
    ($fn_name:ident, $extractor:path, $error_msg:expr $(, $extra_args:expr)*) => {
        pub fn $fn_name(
            &mut self,
            file_path: &str,
            config: &StreamProcessorConfig,
        ) -> Result<ProcessStats, String> {
            use std::cell::RefCell;
            use std::rc::Rc;
            
            let processor_ref = Rc::new(RefCell::new(self));
            let processor_clone = processor_ref.clone();
            let config_clone = config.clone();
            
            $extractor(file_path $(, $extra_args)*, move |text: String| -> Result<bool, String> {
                let mut proc = processor_clone.borrow_mut();
                proc.buffer.push_str(&text);
                proc.buffer.push('\n');
                
                if proc.buffer.len() >= CHUNK_SIZE {
                    if let Err(e) = proc.process_chunk_sync(&config_clone) {
                        return Err(e);
                    }
                }
                
                Ok(true)
            }).map_err(|e| format!("{}: {}", $error_msg, e))?;
            
            {
                let mut proc = processor_ref.borrow_mut();
                if !proc.buffer.is_empty() {
                    proc.process_chunk_sync(config)
                        .map_err(|e| format!("处理剩余缓冲区失败: {}", e))?;
                }
            }
            
            Ok(processor_ref.borrow().stats.clone())
        }
    };
}

impl FileStreamProcessor {
    /// 创建新的流式处理器
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(CHUNK_SIZE + OVERLAP_SIZE),
            previous_overlap: String::new(),
            total_bytes: 0,
            total_chars: 0,
            chunk_index: 0,
            global_line_offset: 0,
            stats: ProcessStats {
                total_bytes: 0,
                total_chars: 0,
                chunks_processed: 0,
                sensitive_count: 0,
                counts: std::collections::HashMap::new(),
                expression_matched: None,
            },
        }
    }

    /// 主入口：流式处理文件
    /// 
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `config` - 处理器配置
    /// * `pre_extracted_text` - 预提取的文本（docx/pdf等需要先解析的文件）
    ///   - None: 直接流式读取原始文件（txt/log/csv等）
    ///   - Some(text): 对已提取的文本进行分块处理
    /// 
    /// # Returns
    /// 处理统计信息
    pub async fn process_file(
        &mut self,
        file_path: &str,
        config: &StreamProcessorConfig,
        pre_extracted_text: Option<String>,
    ) -> Result<ProcessStats, String> {
        if let Some(text) = pre_extracted_text {
            // 路径B: 处理已提取的文本（docx/xlsx/pdf等）
            self.process_extracted_text(&text, config).await?;
        } else {
            // 路径A: 直接流式读取原始文件（txt/log/csv等）
            self.process_raw_file(file_path, config).await?;
        }

        // 处理剩余缓冲区
        if !self.buffer.is_empty() {
            self.process_chunk(config).await?;
        }

        Ok(self.stats.clone())
    }

    impl_streaming_processor!(
        process_pdf_streaming,
        crate::core::parsers::pdf_parser::stream_extract_pdf,
        "PDF流式提取失败"
    );

    impl_streaming_processor!(
        process_excel_streaming,
        crate::core::parsers::office::excel_parser::stream_extract_excel,
        "Excel流式提取失败"
    );

    impl_streaming_processor!(
        process_office_streaming,
        crate::core::parsers::office::msoffice_parser::stream_extract_docx_pptx,
        "Office文件流式提取失败"
    );

    impl_streaming_processor!(
        process_odt_streaming,
        crate::core::parsers::office::opendocument_parser::stream_extract_odt,
        "ODT文件流式提取失败"
    );

    impl_streaming_processor!(
        process_ods_streaming,
        crate::core::parsers::office::opendocument_parser::stream_extract_ods,
        "ODS文件流式提取失败"
    );

    impl_streaming_processor!(
        process_odp_streaming,
        crate::core::parsers::office::opendocument_parser::stream_extract_odp,
        "ODP文件流式提取失败"
    );

    impl_streaming_processor!(
        process_doc_streaming,
        crate::core::parsers::office::msoffice_parser::stream_extract_doc,
        "DOC文件流式提取失败",
        1024 * 1024  // chunk_size: 1MB
    );

    impl_streaming_processor!(
        process_ppt_streaming,
        crate::core::parsers::office::msoffice_parser::stream_extract_ppt,
        "PPT文件流式提取失败",
        1024 * 1024  // chunk_size: 1MB
    );

    impl_streaming_processor!(
        process_rtf_streaming,
        crate::core::parsers::office::rtf_parser::stream_extract_rtf,
        "RTF文件流式提取失败",
        CHUNK_SIZE
    );

    /// 【内部】处理单个块的核心逻辑
    fn process_current_chunk(
        &mut self,
        config: &StreamProcessorConfig,
    ) {
        // 添加上一块的重叠区到当前块开头
        let mut current_chunk = self.previous_overlap.clone();
        current_chunk.push_str(&self.buffer);

        // 检测敏感数据
        if !config.preview_mode {
            let (counts, expr_matched) = detect_sensitive_data(
                &current_chunk,
                &config.enabled_types,
                config.enable_builtin_rules,  // ✅ 从 config 传递
                config.search_expression.as_deref(),  // ✅ 从 config 传递
            );
            
            // 累加总数
            let chunk_sensitive_count: u32 = counts.values().sum();
            self.stats.sensitive_count += chunk_sensitive_count as usize;
            
            // 【新增】合并详细计数
            for (type_id, count) in counts {
                *self.stats.counts.entry(type_id).or_insert(0) += count;
            }
            
            // 【新增】更新表达式匹配状态（OR逻辑：只要有一块匹配就算匹配）
            if let Some(matched) = expr_matched {
                self.stats.expression_matched = Some(
                    self.stats.expression_matched.unwrap_or(0) | matched
                );
            }
        }

        // 计算行数
        let line_count = current_chunk.chars().filter(|&c| c == '\n').count();
        self.global_line_offset += line_count;

        // 更新统计
        self.stats.chunks_processed += 1;
        self.stats.total_chars += current_chunk.len();
        self.stats.total_bytes += self.buffer.len() as u64;

        // 保存当前块的尾部作为下一块的重叠区
        if self.buffer.len() > OVERLAP_SIZE {
            // 【关键修复】确保在字符边界处切割，避免UTF-8多字节字符被截断
            let overlap_start = self.buffer.len() - OVERLAP_SIZE;
            // 找到最近的字符边界（向前查找）
            let char_boundary = self.buffer
                .char_indices()
                .find(|&(idx, _)| idx >= overlap_start)
                .map(|(idx, _)| idx)
                .unwrap_or(overlap_start);
            
            self.previous_overlap = self.buffer[char_boundary..].to_string();
        } else {
            self.previous_overlap = self.buffer.clone();
        }

        // 清空缓冲区
        self.buffer.clear();
        self.chunk_index += 1;
    }

    /// 同步版本的process_chunk（用于流式回调）
    fn process_chunk_sync(
        &mut self,
        config: &StreamProcessorConfig,
    ) -> Result<(), String> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.process_current_chunk(config);
        Ok(())
    }

    /// 处理原始文件（真正的流式读取）
    async fn process_raw_file(
        &mut self,
        file_path: &str,
        config: &StreamProcessorConfig,
    ) -> Result<(), String> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Err(format!("文件不存在: {}", file_path));
        }

        let file = File::open(path).await
            .map_err(|e| format!("无法打开文件 {}: {}", file_path, e))?;
        
        let mut reader = BufReader::new(file);
        let mut chunk_buffer = vec![0u8; CHUNK_SIZE];

        loop {
            // 读取一个块
            let bytes_read = reader.read(&mut chunk_buffer).await
                .map_err(|e| format!("读取文件失败 {}: {}", file_path, e))?;

            if bytes_read == 0 {
                break; // EOF
            }

            // 转换为字符串（使用UTF-8，错误字符替换为）
            let text = String::from_utf8_lossy(&chunk_buffer[..bytes_read]);
            self.buffer.push_str(&text);
            self.total_bytes += bytes_read as u64;

            // 如果缓冲区足够大，处理一个块
            if self.buffer.len() >= CHUNK_SIZE {
                self.process_chunk(config).await?;
            }
        }

        Ok(())
    }

    /// 处理预提取的文本（分块处理）
    async fn process_extracted_text(
        &mut self,
        text: &str,
        config: &StreamProcessorConfig,
    ) -> Result<(), String> {
        let chars: Vec<char> = text.chars().collect();
        let total_len = chars.len();
        let mut start = 0;

        while start < total_len {
            // 计算块的结束位置
            let end = (start + CHUNK_SIZE).min(total_len);
            
            // 提取当前块
            let chunk: String = chars[start..end].iter().collect();
            self.buffer.push_str(&chunk);
            self.total_bytes += chunk.len() as u64; // 近似值

            // 处理当前块
            self.process_chunk(config).await?;

            start = end;
        }

        Ok(())
    }

    /// 处理单个块
    async fn process_chunk(&mut self, config: &StreamProcessorConfig) -> Result<(), String> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.process_current_chunk(config);
        Ok(())
    }

    /// 获取处理统计
    #[allow(dead_code)]
    pub fn get_stats(&self) -> &ProcessStats {
        &self.stats
    }

    /// 重置处理器状态
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.previous_overlap.clear();
        self.total_bytes = 0;
        self.total_chars = 0;
        self.chunk_index = 0;
        self.global_line_offset = 0;
        self.stats = ProcessStats {
            total_bytes: 0,
            total_chars: 0,
            chunks_processed: 0,
            sensitive_count: 0,
            counts: std::collections::HashMap::new(),
            expression_matched: None,
        };
    }
}

impl Default for FileStreamProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_process_small_file() {
        // 创建临时小文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "这是一段测试文本").unwrap();
        writeln!(temp_file, "包含一些敏感信息：13800138000").unwrap();
        
        let config = StreamProcessorConfig {
            enabled_types: vec!["phone".to_string()],
            preview_mode: false,
            enable_builtin_rules: true,
            search_expression: None,
        };

        let mut processor = FileStreamProcessor::new();
        let stats = processor.process_file(
            temp_file.path().to_str().unwrap(),
            &config,
            None,
        ).await.unwrap();

        assert!(stats.chunks_processed >= 1);
        assert!(stats.total_chars > 0);
    }

    #[tokio::test]
    async fn test_process_large_file() {
        // 创建临时大文件（10MB）
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_text = "A".repeat(10 * 1024 * 1024); // 10MB
        write!(temp_file, "{}", large_text).unwrap();
        
        let config = StreamProcessorConfig {
            enabled_types: vec![],
            preview_mode: true,
            enable_builtin_rules: true,
            search_expression: None,
        };

        let mut processor = FileStreamProcessor::new();
        let stats = processor.process_file(
            temp_file.path().to_str().unwrap(),
            &config,
            None,
        ).await.unwrap();

        // 应该分成多个块处理
        assert!(stats.chunks_processed >= 2);
        assert_eq!(stats.total_bytes, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_process_extracted_text() {
        let text = "测试文本\n第二行\n第三行".to_string();
        
        let config = StreamProcessorConfig {
            enabled_types: vec![],
            preview_mode: true,
            enable_builtin_rules: true,
            search_expression: None,
        };

        let mut processor = FileStreamProcessor::new();
        let stats = processor.process_file(
            "dummy.txt",
            &config,
            Some(text),
        ).await.unwrap();

        assert!(stats.chunks_processed >= 1);
        assert!(stats.total_chars > 0);
    }

    #[tokio::test]
    async fn test_overlap_handling() {
        // 测试跨边界敏感词检测
        let mut temp_file = NamedTempFile::new().unwrap();
        
        // 创建一个刚好在边界处的手机号
        let padding = "A".repeat(CHUNK_SIZE - 10);
        let phone = "13800138000";
        write!(temp_file, "{}{}", padding, phone).unwrap();
        
        let config = StreamProcessorConfig {
            enabled_types: vec!["phone".to_string()],
            preview_mode: false,
            enable_builtin_rules: true,
            search_expression: None,
        };

        let mut processor = FileStreamProcessor::new();
        let stats = processor.process_file(
            temp_file.path().to_str().unwrap(),
            &config,
            None,
        ).await.unwrap();

        // 应该能检测到跨越边界的手机号
        assert!(stats.sensitive_count > 0, "应该检测到跨边界的敏感数据");
    }

    #[tokio::test]
    async fn test_reset() {
        let mut processor = FileStreamProcessor::new();
        processor.buffer = "test".to_string();
        processor.total_bytes = 100;
        
        processor.reset();
        
        assert!(processor.buffer.is_empty());
        assert_eq!(processor.total_bytes, 0);
        assert_eq!(processor.chunk_index, 0);
    }

    #[tokio::test]
    async fn test_detailed_counts() {
        // 创建包含多种敏感信息的临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "测试文件包含多种敏感信息:").unwrap();
        writeln!(temp_file, "手机号: 13800138000").unwrap();
        writeln!(temp_file, "邮箱: test@example.com").unwrap();
        writeln!(temp_file, "另一个手机: 13900139000").unwrap();
        
        let config = StreamProcessorConfig {
            enabled_types: vec!["phone".to_string(), "email".to_string()],
            preview_mode: false,
            enable_builtin_rules: true,
            search_expression: None,
        };

        let mut processor = FileStreamProcessor::new();
        let stats = processor.process_file(
            temp_file.path().to_str().unwrap(),
            &config,
            None,
        ).await.unwrap();

        // 验证总数
        assert_eq!(stats.sensitive_count, 3, "应该检测到3处敏感信息");
        
        // 【新增】验证详细计数
        assert_eq!(stats.counts.get("phone"), Some(&2), "应该检测到2个手机号");
        assert_eq!(stats.counts.get("email"), Some(&1), "应该检测到1个邮箱");
        
        // 【新增】验证表达式匹配状态（未配置表达式应为None）
        assert_eq!(stats.expression_matched, None, "未配置表达式时应为None");
    }

    #[tokio::test]
    async fn test_expression_matching() {
        // 创建包含关键字的临时文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "这是一个测试文件").unwrap();
        writeln!(temp_file, "包含关键字: 密码").unwrap();
        writeln!(temp_file, "另一行文本").unwrap();
        
        let config = StreamProcessorConfig {
            enabled_types: vec![],  // 不启用内置规则
            preview_mode: false,
            enable_builtin_rules: false,  // 禁用内置规则
            search_expression: Some("密码".to_string()),  // 使用自定义表达式
        };

        let mut processor = FileStreamProcessor::new();
        let stats = processor.process_file(
            temp_file.path().to_str().unwrap(),
            &config,
            None,
        ).await.unwrap();

        // 验证表达式匹配状态
        assert_eq!(stats.expression_matched, Some(1), "应该匹配到表达式");
        
        // 禁用内置规则时，counts应为空
        assert!(stats.counts.is_empty(), "禁用内置规则时counts应为空");
    }
}
