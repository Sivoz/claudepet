use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::claude::JsonlEntry;

/// JSONL 增量解析器
/// 维护每个文件的已读偏移量，只解析新增部分
pub struct IncrementalParser {
    /// 每个文件的已读字节偏移量
    offsets: HashMap<PathBuf, u64>,
    /// 每个文件未完整的行缓冲（处理读到半行的情况）
    buffers: HashMap<PathBuf, String>,
}

impl IncrementalParser {
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    /// 解析文件中新增的行，返回解析出的 JSONL 条目
    pub fn parse_new_entries(&mut self, path: &Path) -> Vec<JsonlEntry> {
        let mut entries = Vec::new();

        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("Failed to open {:?}: {}", path, e);
                return entries;
            }
        };

        // 跳到上次读取的位置
        let offset = *self.offsets.get(path).unwrap_or(&0);
        if let Err(e) = file.seek(SeekFrom::Start(offset)) {
            log::warn!("Failed to seek {:?}: {}", path, e);
            return entries;
        }

        // 读取新增内容
        let mut new_data = String::new();
        if let Err(e) = file.read_to_string(&mut new_data) {
            log::warn!("Failed to read {:?}: {}", path, e);
            return entries;
        }

        if new_data.is_empty() {
            return entries;
        }

        // 拼接上次未完整的行
        let buf = self.buffers.remove(path).unwrap_or_default();
        let combined = buf + &new_data;

        let mut bytes_consumed: usize = 0;

        for line in combined.split('\n') {
            bytes_consumed += line.len() + 1; // +1 for '\n'

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<JsonlEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    // 如果是最后一行且没有换行符结尾，可能是半行
                    if !combined.ends_with('\n')
                        && bytes_consumed >= combined.len()
                    {
                        self.buffers.insert(path.to_path_buf(), line.to_string());
                    } else {
                        log::debug!("Failed to parse JSONL line in {:?}: {}", path, e);
                    }
                }
            }
        }

        // 更新偏移量（基于实际新读取的字节数，减去缓冲的部分）
        let buffered_len = self
            .buffers
            .get(path)
            .map(|b| b.len() as u64)
            .unwrap_or(0);
        let new_offset = offset + new_data.len() as u64 - buffered_len;
        self.offsets.insert(path.to_path_buf(), new_offset);

        entries
    }

    /// 移除已删除文件的跟踪
    pub fn remove_file(&mut self, path: &Path) {
        self.offsets.remove(path);
        self.buffers.remove(path);
    }
}
