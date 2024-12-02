use std::{fs::read_to_string, io::Error};
use super::line::Line;
use super::Location;

#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<Line>
}

impl Buffer {

    // 读取文件内容到buffer中
    pub fn load(file_name: &str) -> Result<Self, Error> {
        let contents = read_to_string(file_name)?;
        let lines = contents.lines()
            .map(|value| Line::from(value))
            .collect();

        Ok(Self{ lines })
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
        if at.line_index > self.lines.len() {
            return;
        }
        if at.line_index == self.lines.len() {
            self.lines.push(Line::from(&character.to_string()));
        } else if let Some(line) = self.lines.get_mut(at.line_index) {
            line.insert_char(character, at.grapheme_index);
        }
    }
    
    pub fn delete(&mut self, at: Location) {
        if let Some(line) = self.lines.get(at.line_index) {
            // 如果删除位置位于当前行的末尾且不是文件的最后一行，
            // 则需要将下一行连接到当前行上，即合并两行。
            if at.grapheme_index >= line.grapheme_count()
                && self.lines.len() > at.line_index.saturating_add(1)
            {
                // 移除下一行并将其内容附加到当前行
                let next_line = self.lines.remove(at.line_index.saturating_add(1));
                // 安全性：由于我们已经检查了下一行的存在，因此可以安全地使用索引访问。
                #[allow(clippy::indexing_slicing)]
                self.lines[at.line_index].append(&next_line);
            } else if at.grapheme_index < line.grapheme_count() {
                // 删除指定位置的字符
                #[allow(clippy::indexing_slicing)]
                self.lines[at.line_index].delete(at.grapheme_index);
            }
            // 如果删除位置超出了当前行的长度，但没有下一行可合并，则不做任何操作
        }
    }
}