use std::{fs::{read_to_string, File}, io::{Error, Write}};
use crate::editor::fileinfo::FileInfo;

use super::line::Line;
use super::Location;

#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<Line>,
    pub file_info: FileInfo,
    // dirty 标志表示缓冲区是否已被修改。此文件中的所有其他更改旨在在插入时将 dirty 切换为 true。
    pub dirty: bool,
}

impl Buffer {

    // 读取文件内容到buffer中
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

    // 保存内容到本地
    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(path) = &self.file_info.path {
            let mut file = File::create(path)?;
            for line in &self.lines {
                writeln!(file, "{line}")?;
            }
            self.dirty = false;
        }
        Ok(())
    }

    // buffer是否为空
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    // 插入字符
    pub fn insert_char(&mut self, character: char, at: Location) {
        if at.line_index > self.height() {
            return;
        }
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