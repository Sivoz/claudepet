use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::claude::JsonlEntry;

/// 行缓冲上限（1 MB），超过则丢弃防止 OOM
const MAX_LINE_BYTES: usize = 1024 * 1024;

/// 每个文件的解析状态
struct FileState {
    offset: u64,
    #[cfg(unix)]
    inode: u64,
    line_buf: String,
}

impl FileState {
    #[cfg(unix)]
    fn new(inode: u64) -> Self {
        Self {
            offset: 0,
            inode,
            line_buf: String::new(),
        }
    }

    #[cfg(not(unix))]
    fn new() -> Self {
        Self {
            offset: 0,
            line_buf: String::new(),
        }
    }
}

/// JSONL 增量解析器
/// 维护每个文件的已读偏移量、inode、行缓冲，只解析新增部分
pub struct IncrementalParser {
    files: HashMap<PathBuf, FileState>,
}

impl IncrementalParser {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// 解析文件中新增的行，返回解析出的 JSONL 条目
    pub fn parse_new_entries(&mut self, path: &Path) -> Vec<JsonlEntry> {
        let mut entries = Vec::new();

        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(?path, "Failed to open file: {}", e);
                return entries;
            }
        };

        let meta = match file.metadata() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(?path, "Failed to get metadata: {}", e);
                return entries;
            }
        };

        let file_size = meta.len();

        // 获取或创建文件状态，检测 inode 变化和截断
        #[cfg(unix)]
        let current_inode = {
            use std::os::unix::fs::MetadataExt;
            meta.ino()
        };

        let state = self
            .files
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                #[cfg(unix)]
                { FileState::new(current_inode) }
                #[cfg(not(unix))]
                { FileState::new() }
            });

        // 检测文件替换（inode 变化）或截断（size < offset）
        #[cfg(unix)]
        if state.inode != current_inode {
            tracing::warn!(?path, old_inode = state.inode, new_inode = current_inode, "file replaced (inode changed), resetting");
            *state = FileState::new(current_inode);
        }

        if file_size < state.offset {
            tracing::warn!(?path, old_offset = state.offset, new_size = file_size, "file truncated, resetting");
            state.offset = 0;
            state.line_buf.clear();
        }

        // 行缓冲上限保护
        if state.line_buf.len() > MAX_LINE_BYTES {
            tracing::error!(?path, buf_size = state.line_buf.len(), "line buffer exceeded 1MB, discarding");
            state.line_buf.clear();
        }

        // 跳到上次读取的位置
        if let Err(e) = file.seek(SeekFrom::Start(state.offset)) {
            tracing::warn!(?path, "Failed to seek: {}", e);
            return entries;
        }

        // 读取新增内容
        let mut new_data = String::new();
        if let Err(e) = file.read_to_string(&mut new_data) {
            tracing::warn!(?path, "Failed to read: {}", e);
            return entries;
        }

        if new_data.is_empty() {
            return entries;
        }

        // 拼接上次未完整的行
        let buf = std::mem::take(&mut state.line_buf);
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
                        state.line_buf = line.to_string();
                    } else {
                        tracing::debug!(?path, "Failed to parse JSONL line: {}", e);
                    }
                }
            }
        }

        // 更新偏移量
        state.offset += new_data.len() as u64;

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp_jsonl(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn incremental_read_resumes_from_offset() {
        let mut parser = IncrementalParser::new();
        let mut f = tempfile::NamedTempFile::new().unwrap();

        // 写入第一条
        writeln!(f, r#"{{"type":"user","message":{{"role":"user"}}}}"#).unwrap();
        f.flush().unwrap();

        let entries = parser.parse_new_entries(f.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type.as_deref(), Some("user"));

        // 追加第二条
        writeln!(f, r#"{{"type":"assistant","message":{{"role":"assistant"}}}}"#).unwrap();
        f.flush().unwrap();

        let entries = parser.parse_new_entries(f.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type.as_deref(), Some("assistant"));
    }

    #[test]
    fn malformed_line_is_skipped() {
        let mut parser = IncrementalParser::new();
        let f = write_tmp_jsonl(&[
            r#"{"type":"user","message":{"role":"user"}}"#,
            r#"NOT VALID JSON"#,
            r#"{"type":"assistant","message":{"role":"assistant"}}"#,
        ]);

        let entries = parser.parse_new_entries(f.path());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type.as_deref(), Some("user"));
        assert_eq!(entries[1].entry_type.as_deref(), Some("assistant"));
    }

    #[test]
    fn truncated_tail_is_buffered_for_next_read() {
        let mut parser = IncrementalParser::new();
        let mut f = tempfile::NamedTempFile::new().unwrap();

        // 写入一行完整的 + 一行不完整的（无换行）
        writeln!(f, r#"{{"type":"user","message":{{"role":"user"}}}}"#).unwrap();
        write!(f, r#"{{"type":"assistant","message":{{"role":"assi"#).unwrap();
        f.flush().unwrap();

        let entries = parser.parse_new_entries(f.path());
        assert_eq!(entries.len(), 1); // 只解析出完整行

        // 补全截断行
        writeln!(f, r#"stant"}}}}"#).unwrap();
        f.flush().unwrap();

        let entries = parser.parse_new_entries(f.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type.as_deref(), Some("assistant"));
    }

    #[test]
    fn empty_file_returns_empty() {
        let mut parser = IncrementalParser::new();
        let f = tempfile::NamedTempFile::new().unwrap();
        let entries = parser.parse_new_entries(f.path());
        assert!(entries.is_empty());
    }

    #[test]
    fn blank_lines_are_ignored() {
        let mut parser = IncrementalParser::new();
        let f = write_tmp_jsonl(&[
            "",
            r#"{"type":"user","message":{"role":"user"}}"#,
            "",
            "",
        ]);
        let entries = parser.parse_new_entries(f.path());
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn truncation_resets_offset() {
        let mut parser = IncrementalParser::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");

        // 写入较长内容，让 offset 推进
        let long_line = format!("{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"{}\" }}}}\n", "x".repeat(200));
        std::fs::write(&path, &long_line).unwrap();
        let entries = parser.parse_new_entries(&path);
        assert_eq!(entries.len(), 1);

        // 截断文件（写入短得多的内容，size < offset）
        std::fs::write(&path, "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\"}}\n").unwrap();
        let entries = parser.parse_new_entries(&path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type.as_deref(), Some("assistant"));
    }

    #[test]
    fn file_rotation_resets_offset() {
        let mut parser = IncrementalParser::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");

        // 写入第一个文件
        std::fs::write(&path, "{\"type\":\"user\",\"message\":{\"role\":\"user\"}}\n").unwrap();
        let entries = parser.parse_new_entries(&path);
        assert_eq!(entries.len(), 1);

        // 删除并重新创建（模拟 rotation，新 inode）
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\"}}\n").unwrap();
        let entries = parser.parse_new_entries(&path);
        // 在 Unix 上会检测到 inode 变化，在其他平台会因为截断检测重置
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type.as_deref(), Some("assistant"));
    }

    #[test]
    fn oversized_line_buffer_is_discarded() {
        let mut parser = IncrementalParser::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");

        // 写入超长不完整行（>1MB，无换行结尾）
        let big_line = "x".repeat(MAX_LINE_BYTES + 100);
        std::fs::write(&path, &big_line).unwrap();

        // 第一次读取：行缓冲中存入大数据
        let entries = parser.parse_new_entries(&path);
        assert!(entries.is_empty());

        // 下次读取前会检测到 line_buf > MAX_LINE_BYTES 并清除
        // 追加一条正常数据
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "\n{{\"type\":\"user\",\"message\":{{\"role\":\"user\"}}}}").unwrap();
        drop(f);

        let entries = parser.parse_new_entries(&path);
        // 大行被丢弃，正常行应该被解析出来
        assert_eq!(entries.len(), 1);
    }
}
