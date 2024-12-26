use std::{fs::{read_to_string, File}, io::{Error, Write}};
use super::FileInfo;
use super::Line;
use crate::prelude::*;

#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<Line>,
    pub file_info: FileInfo,
    // dirty 标志表示缓冲区是否已被修改。此文件中的所有其他更改旨在在插入时将 dirty 切换为 true。
    pub dirty: bool,
}

impl Buffer {

    /// 读取文件内容到buffer中
    pub fn load(file_name: &str) -> Result<Self, Error> {
        let contents = read_to_string(file_name)?;
        let lines = contents.lines()
            .map(|value| Line::from(value))
            .collect();

        Ok(Self{
            lines,
            file_info: FileInfo::from(file_name),
            dirty: false,
        })
    }

    /// 向下搜索给定查询字符串的位置。
    ///
    /// # 参数
    /// - `query`: 要搜索的字符串。
    /// - `from`: 搜索的起始位置（行索引和字素索引）。
    ///
    /// # 返回值
    /// 如果找到匹配项，则返回匹配项的位置；否则返回 `None`。
    ///
    /// # 逻辑说明
    /// 该方法从指定位置开始向下搜索，直到文档末尾，然后环绕回文档顶部继续搜索，
    /// 确保当前行被搜索两次（一次从中点开始，一次从行首开始），以捕捉所有可能的匹配。
    pub fn search_forward(&self, query: &str, from: Location) -> Option<Location> {
        if query.is_empty() {
            return None;
        }
        // 标记是否是第一次处理当前行
        let mut is_first = true;

        for (line_index, line) in self
            .lines
            .iter()
            .enumerate()
            // 遍历文档中的每一行，并允许循环遍历（即当到达最后一行后，继续从第一行开始）
            .cycle()
            .skip(from.line_index)
            // 为了确保当前行被搜索两次（一次从中点开始，一次从行首开始），多取一行
            .take(self.lines.len().saturating_add(1))
        {
            // 确定当前行的起始字素索引：
            // - 如果是第一次处理当前行，则从 `from.grapheme_index` 开始；
            // - 否则，从行首（索引为0）开始。
            let from_grapheme_index = if is_first {
                is_first = false;
                from.grapheme_index
            } else {
                0
            };

            // 在当前行中搜索查询字符串，如果找到匹配项，则返回匹配位置。
            if let Some(grapheme_index) = line.search_forward(query, from_grapheme_index) {
                return Some(Location {
                    grapheme_index,
                    line_index,
                });
            }
        }
        None
    }

    /// 向上搜索给定查询字符串的位置。
    ///
    /// # 参数
    /// - `query`: 要搜索的字符串。
    /// - `from`: 搜索的起始位置（行索引和字素索引）。
    ///
    /// # 返回值
    /// 如果找到匹配项，则返回匹配项的位置；否则返回 `None`。
    ///
    /// # 逻辑说明
    /// 该方法从指定位置开始向上搜索，直到文档顶部，然后环绕回文档底部继续搜索，
    /// 确保当前行被搜索两次（一次从中点开始，一次从行尾开始），以捕捉所有可能的匹配。
    pub fn search_backward(&self, query: &str, from: Location) -> Option<Location> {
        if query.is_empty() {
            return None;
        }
        // 标记是否是第一次处理当前行
        let mut is_first = true;

        for (line_index, line) in self
            .lines
            .iter()
            .enumerate()
            // 反转迭代器，从最后一行开始向上遍历
            .rev()
            .cycle()
            // 跳过起始位置之后的所有行，并确保不会越界。
            .skip(self.lines.len().saturating_sub(from.line_index).saturating_sub(1))
            // 为了确保当前行被搜索两次（一次从中点开始，一次从行尾开始），多取一行
            .take(self.lines.len().saturating_add(1))
        {
            // 确定当前行的起始字素索引：
            // - 如果是第一次处理当前行，则从 `from.grapheme_index` 开始；
            // - 否则，从行尾（即最后一个字素索引）开始。
            let from_grapheme_index = if is_first {
                is_first = false;
                from.grapheme_index
            } else {
                line.grapheme_count()
            };
            // 在当前行中反向搜索查询字符串，如果找到匹配项，则返回匹配位置。
            if let Some(grapheme_index) = line.search_backward(query, from_grapheme_index) {
                return Some(Location {
                    grapheme_index,
                    line_index,
                });
            }
        }
        None
    }

    /// 保存文件内容
    fn save_to_file(&self, file_info: &FileInfo) -> Result<(), Error> {
        if let Some(path) = file_info.get_path() {
            let mut file = File::create(path)?;
            for line in &self.lines {
                writeln!(file, "{line}")?;
            }
        }
        Ok(())
    }

    /// 另存为
    pub fn save_as(&mut self, file_name: &str) -> Result<(), Error> {
        let file_info = FileInfo::from(file_name);
        self.save_to_file(&file_info)?;
        self.file_info = file_info;
        self.dirty = false;
        Ok(())
    }
    
    /// 保存现有文件
    pub fn save(&mut self) -> Result<(), Error> {
        self.save_to_file(&self.file_info)?;
        self.dirty = false;
        Ok(())
    }

    /// buffer是否为空
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// 是否已加载文件
    pub const fn is_file_loaded(&self) -> bool {
        self.file_info.has_path()
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    // 插入字符
    pub fn insert_char(&mut self, character: char, at: Location) {
        // if at.line_index > self.height() {
        //     return;
        // }
        debug_assert!(at.line_index <= self.height());
        if at.line_index == self.height() {
            self.lines.push(Line::from(&character.to_string()));
            self.dirty = true;
        } else if let Some(line) = self.lines.get_mut(at.line_index) {
            line.insert_char(character, at.grapheme_index);
            self.dirty = true;
        }
    }
    
    pub fn delete(&mut self, at: Location) {
        if let Some(line) = self.lines.get(at.line_index) {
            // 如果删除位置位于当前行的末尾且不是文件的最后一行，
            // 则需要将下一行连接到当前行上，即合并两行。
            if at.grapheme_index >= line.grapheme_count()
                && self.height() > at.line_index.saturating_add(1)
            {
                // 移除下一行并将其内容附加到当前行
                let next_line = self.lines.remove(at.line_index.saturating_add(1));
                // 安全性：由于我们已经检查了下一行的存在，因此可以安全地使用索引访问。
                #[allow(clippy::indexing_slicing)]
                self.lines[at.line_index].append(&next_line);
                self.dirty = true;
            } else if at.grapheme_index < line.grapheme_count() {
                // 删除指定位置的字符
                #[allow(clippy::indexing_slicing)]
                self.lines[at.line_index].delete(at.grapheme_index);
                self.dirty = true;
            }
            // 如果删除位置超出了当前行的长度，但没有下一行可合并，则不做任何操作
        }
    }

    pub fn insert_newline(&mut self, at: Location) {
        if at.line_index == self.height() {
            self.lines.push(Line::default());
            self.dirty = true;
        } else if let Some(line) = self.lines.get_mut(at.line_index) {
            let new = line.split(at.grapheme_index);
            self.lines.insert(at.line_index.saturating_add(1), new);
            self.dirty = true;
        }
    }
}