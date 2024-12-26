use crate::prelude::*;
use super::super::{DocumentStatus, Terminal};
use super::UIComponent;

#[derive(Default)]
pub struct StatusBar {
    // 当前保存状态
    current_status: DocumentStatus,
    // 是否需要重新渲染
    needs_redraw: bool,
    size: Size
}

impl StatusBar {
    // 更新状态
    pub fn update_status(&mut self, new_status: DocumentStatus) {
        if new_status != self.current_status {
            self.current_status = new_status;
            self.needs_redraw = true;
        }
    }
}

impl UIComponent for StatusBar {
    fn set_needs_redraw(&mut self, value: bool) {
        self.needs_redraw = value
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    fn set_size(&mut self, size: Size) {
        self.size = size
    }

    fn draw(&mut self, origin_row: RowIdx) -> Result<(), std::io::Error> {
        // 组装状态栏的第一部分：文件名、行数和是否修改的指示符
        let line_count = self.current_status.line_count_to_string();
        let modified_indicator = self.current_status.modified_indicator_to_string();
        let beginning = format!(
            "{} - {line_count} {modified_indicator}",
            self.current_status.file_name
        );

        // 组装整个状态栏，在末尾加上位置指示符
        let position_indicator = self.current_status.position_indicator_to_string();
        // 计算剩余空间的长度，确保状态栏内容不会超出终端宽度
        let remainder_len = self.size.width.saturating_sub(beginning.len());
        // 使用格式化字符串将所有部分组合起来，确保位置指示符靠右对齐
        let status = format!("{beginning}{position_indicator:>remainder_len$}");
        
        // 只有当状态栏内容完全适合终端宽度时才打印；否则，打印空字符串以清除该行
        let to_print = if status.len() <= self.size.width {
            status
        } else {
            String::new()
        };
        // 在指定的位置打印倒置颜色的状态栏行
        Terminal::print_inverted_row(origin_row, &to_print)?;

        Ok(())
    }
}