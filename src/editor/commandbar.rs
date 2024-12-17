use std::{cmp::min, io::Error};
use super::{command::Edit, line::Line, Size, Terminal, UIComponent};

#[derive(Default)]
pub struct CommandBar {
    /// 提示符字符串，显示在命令栏的开头。
    prompt: String,
    /// 当前输入的内容值
    value: Line,
    needs_redraw: bool,
    size: Size,
}

impl CommandBar {

    /// 处理编辑命令
    pub fn handle_edit_command(&mut self, command: Edit) {
        match command {
            Edit::Insert(character) => self.value.append_char(character),
            Edit::Delete | Edit::InsertNewline=> {}
            Edit::DeleteBackward => self.value.delete_last(),
        }
        self.set_needs_redraw(true);
    }

    /// 获取插入符(光标对应列位置)
    /// 
    /// 插入符号的 x 位置（它所在的列）是输入内容宽度加上提示符的长度，
    /// 假设 `self.prompt` 仅由 ASCII 字符组成。或者它是终端的宽度（即终端的最右侧），
    /// 取两者中的较小值。
    pub fn caret_position_col(&self) -> usize {
        
        let max_width = self
            .prompt
            .len()
            .saturating_add(self.value.grapheme_count());
        min(max_width, self.size.width)
    }

    /// 获取命令栏的当前值的字符串
    pub fn value(&self) -> String {
        self.value.to_string()
    }

    /// 设置命令栏的提示符
    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
        self.set_needs_redraw(true);
    }

    /// 清空命令栏的值
    pub fn clear_value(&mut self) {
        self.value = Line::default();
        self.set_needs_redraw(true);
    }
}
impl UIComponent for CommandBar {

    fn set_needs_redraw(&mut self, value: bool) {
        self.needs_redraw = value;
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    fn draw(&mut self, origin: usize) -> Result<(), Error> {
        // 计算用于显示输入值的空间大小，等于终端宽度减去提示符长度。
        let area_for_value = self.size.width.saturating_sub(self.prompt.len());
        // 获取命令栏值的宽度（以图形符号为单位）。
        let value_end = self.value.width();
        // 计算要显示的命令栏值的起始位置，确保始终显示值的左侧部分。
        let value_start = value_end.saturating_sub(area_for_value);
        // 创建最终要显示的消息，包含提示符和截取后的命令栏值。
        let message = format!(
            "{}{}",
            self.prompt,
            self.value.get_visible_graphemes(value_start..value_end)
        );
        // 确保消息不会超出终端宽度，如果超出则打印空字符串以清空该行。
        let to_print = if message.len() <= self.size.width {
            message
        } else {
            String::new()
        };
        // 打印到指定行
        Terminal::print_row(origin, &to_print)
    }
}