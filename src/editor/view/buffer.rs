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
        if let Some(line) = self.lines.get_mut(at.line_index) {
            line.delete(at.grapheme_index);
        }
    }
}