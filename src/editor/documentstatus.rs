use crate::prelude::*;

#[derive(Default, Eq, PartialEq, Debug)]
pub struct DocumentStatus {
    pub total_lines: usize,
    pub current_line_index: LineIdx,
    pub is_modified: bool,
    pub file_name: String,
}

impl DocumentStatus {
    // 修改标志
    pub fn modified_indicator_to_string(&self) -> String {
        if self.is_modified {
            String::from("(modified)")
        } else {
            String::new()
        }
    }

    // 总行数展示
    pub fn line_count_to_string(&self) -> String {
        format!("{} lines", self.total_lines)
    }

    // 当前光标/操作位置展示
    pub fn position_indicator_to_string(&self) -> String {
        format!(
            "{}/{}",
            self.current_line_index.saturating_add(1),
            self.total_lines
        )
    }
}