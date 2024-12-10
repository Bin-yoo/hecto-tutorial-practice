use std::{cmp::min, io::Error, str};
use super::{command::{Edit, Move}, documentstatus::DocumentStatus, terminal::{Position, Size, Terminal}, uicomponent::UIComponent, NAME, VERSION};
use buffer::Buffer;
use line::Line;

mod buffer;
mod line;

#[derive(Copy, Clone, Default)]
pub struct Location {
    pub grapheme_index: usize,
    pub line_index: usize,
}

#[derive(Default)]
pub struct View {
    // 存储文本内容的缓冲区
    buffer: Buffer,
    // 标记是否需要重新渲染
    needs_redraw: bool,
    // View总是从 (0, 0) 开始。size 属性决定了可见区域。
    size: Size,
    // 文档中位置
    text_location: Location,
    // view的偏移
    scroll_offset: Position,
}

impl View {

    // 获取状态
    pub fn get_status(&self) -> DocumentStatus {
        DocumentStatus {
            total_lines: self.buffer.height(),
            current_line_index: self.text_location.line_index,
            file_name: format!("{}", self.buffer.file_info),
            is_modified: self.buffer.dirty,
        }
    }

    /// 处理编辑命令。
    ///
    /// # 参数
    /// - `command`: 编辑命令枚举。
    pub fn handle_edit_command(&mut self, command: Edit) {
        match command {
            Edit::Insert(character) => self.insert_char(character),
            Edit::Delete => self.delete(),
            Edit::DeleteBackward => self.delete_backward(),
            Edit::InsertNewline => self.insert_newline(),
        }
    }

    /// 处理移动命令。
    ///
    /// # 参数
    /// - `command`: 移动命令枚举。
    pub fn handle_move_command(&mut self, command: Move) {
        let Size { height, .. } = self.size;
        match command {
            Move::Up => self.move_up(1),
            Move::Down => self.move_down(1),
            Move::Left => self.move_left(),
            Move::Right => self.move_right(),
            Move::PageUp => self.move_up(height.saturating_sub(1)),
            Move::PageDown => self.move_down(height.saturating_sub(1)),
            Move::StartOfLine => self.move_to_start_of_line(),
            Move::EndOfLine => self.move_to_end_of_line(),
        }

        // 处理滚动显示位置
        self.scroll_text_location_into_view();
    }

    // region: file i/o
    // 文件io处理代码区域

    /// 读取文件内容并加载到缓冲区。
    ///
    /// # 参数
    /// - `file_name`: 要加载的文件名。
    ///
    /// 如果文件加载成功，则将其内容保存到缓冲区，并标记视图需要重新渲染。
    pub fn load(&mut self, file_name: &str) -> Result<(), Error> {
        let buffer = Buffer::load(file_name)?;
        self.buffer = buffer;
        self.set_needs_redraw(true);
        Ok(())
    }

    /// 保存缓冲区内容到文件
    pub fn save(&mut self) -> Result<(), Error> {
        self.buffer.save()
    }

    // 文件io处理代码区域结束

    // region: Text editing
    // 文本编辑代码区域

    fn insert_newline(&mut self) {
        self.buffer.insert_newline(self.text_location);
        self.handle_move_command(Move::Right);
        self.set_needs_redraw(true);
    }

    fn delete_backward(&mut self) {
        // 确保我们只在文档贯标不位于左上角时向左移动。
        if self.text_location.line_index != 0 || self.text_location.grapheme_index != 0 {
            self.handle_move_command(Move::Left);
            self.delete();
        }
    }

    fn delete(&mut self) {
        self.buffer.delete(self.text_location);
        self.set_needs_redraw(true);
    }

    // 插入字符
    fn insert_char(&mut self, character: char) {
        // 获取当前所在行的内容长度
        let old_len = self.buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);

        // 在位置上插入字符
        self.buffer.insert_char(character, self.text_location);

        // 获取插入后的长度
        let new_len = self.buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);

        // 正常来说，插入字符后光标要右移一下。这里通过插入前后得长度查来判断
        let grapheme = new_len.saturating_sub(old_len);
        if grapheme > 0 {
            self.handle_move_command(Move::Right);
        }

        self.set_needs_redraw(true);
    }
    // 文本编辑代码区域结束


    // region: Rendering
    // 渲染方法代码


    /// 渲染指定行的内容。
    ///
    /// # 参数
    /// - `at`: 行号，表示要渲染到的目标行。
    /// - `line_text`: 要渲染的文本内容。
    ///
    /// 清除指定行的内容，将文本渲染到该终端行。
    fn render_line(at: usize, line_text: &str) -> Result<(), Error> {
        Terminal::print_row(at, line_text)
    }

    /// 构建欢迎信息字符串，欢迎信息内容会居中显示在终端宽度范围内。
    ///
    /// # 参数
    /// - `width`: 终端的宽度，用于决定欢迎信息的显示位置。
    ///
    /// # 返回值
    /// - 返回一个格式化后的欢迎信息，若终端宽度小于欢迎信息长度，则返回波浪符 "~"。
    fn build_welcome_message(width: usize) -> String {
        if width == 0 {
            return String::new()
        }
        let welcome_message = format!("{NAME} editor -- version {VERSION}");
        let len = welcome_message.len();
        let remaining_width = width.saturating_sub(1);
        // 宽度不够就隐藏隐藏欢迎消息
        if remaining_width < len {
            return "~".to_string();
        }

        format!("{:<1}{:^remaining_width$}", "~", welcome_message)
    }

    // endregion
    // 渲染方法代码结束

    // region: Scrolling
    // view滚动代码块

    // 垂直滚动
    fn scroll_vertically(&mut self, to: usize) {
        let Size { height, .. } = self.size;
        let offset_changed = if to < self.scroll_offset.row {
            // 如果目标行小于当前滚动偏移行，更新滚动偏移行
            self.scroll_offset.row = to;
            true
        } else if to >= self.scroll_offset.row.saturating_add(height) {
            // 如果目标行大于等于当前滚动偏移行加上窗口高度，更新滚动偏移行
            self.scroll_offset.row = to.saturating_sub(height).saturating_add(1);
            true
        } else {
            // 如果目标行在当前滚动偏移行和窗口高度之间，滚动偏移行不变
            false
        };

        // 如果滚动偏移行发生变化，需要重新渲染
        if offset_changed {
            self.needs_redraw = true
        }
    }

    // 水平滚动
    fn scroll_horizontally(&mut self, to: usize) {
        let Size { width, .. } = self.size;
        let offset_changed = if to < self.scroll_offset.col {
            // 如果目标列小于当前滚动偏移列，更新滚动偏移列
            self.scroll_offset.col = to;
            true
        } else if to >= self.scroll_offset.col.saturating_add(width) {
            // 如果目标列大于等于当前滚动偏移列加上窗口宽度，更新滚动偏移列
            self.scroll_offset.col = to.saturating_sub(width).saturating_add(1);
            true
        } else {
            // 如果目标列在当前滚动偏移列和窗口宽度之间，滚动偏移列不变
            false
        };
        
        if offset_changed {
            self.needs_redraw = true
        }
    }

    // 滚动至文本内容位置
    fn scroll_text_location_into_view(&mut self) {
        let Position { row, col } = self.text_location_to_position();
        self.scroll_vertically(row);
        self.scroll_horizontally(col);
    }
    // endregion
    // view滚动代码结束

    // region: Location and Position Handling
    // 处理位置代码

    // 指针位置
    pub fn caret_position(&self) -> Position {
        self.text_location_to_position()
            .saturating_sub(self.scroll_offset)
    }

    // 文本内容位置
    fn text_location_to_position(&self) -> Position {
        let row = self.text_location.line_index;
        let col = self.buffer.lines.get(row).map_or(0, |line| {
            // 获取当前行的图形单元宽度，直到文本位置的图形单元索引
            line.width_until(self.text_location.grapheme_index)
        });
        Position { col, row }
    }
    // endregion
    // 处理位置代码结束

    // region: text location movement
    // 文本位置移动代码

    // 向上移动指定行数
    fn move_up(&mut self, step: usize) {
        self.text_location.line_index = self.text_location.line_index.saturating_sub(step);
        // 确保图形单元索引有效
        self.snap_to_valid_grapheme();
    }

    // 向下移动指定行数
    fn move_down(&mut self, step: usize) {
        self.text_location.line_index = self.text_location.line_index.saturating_add(step);
        // 确保图形单元索引有效
        self.snap_to_valid_grapheme();
        // 确保行索引有效
        self.snap_to_valid_line();
    }


    // 向右移动
    // clippy::arithmetic_side_effects: 这个函数执行算术计算，并且已经显式检查了目标值将在范围内。
    #[allow(clippy::arithmetic_side_effects)]
    fn move_right(&mut self) {
        // 获取当前行的图形单元长度
        let line_width = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);
        if self.text_location.grapheme_index < line_width {
            // 小于长度,则向右移动一个图形单元
            self.text_location.grapheme_index += 1;
        } else {
            // 否则移动到下一行的开头
            self.move_to_start_of_line();
            self.move_down(1);
        }
    }

    // 向左移动
    #[allow(clippy::arithmetic_side_effects)]
    fn move_left(&mut self) {
        if self.text_location.grapheme_index > 0 {
            // 向左移动一个图形单元
            self.text_location.grapheme_index -= 1;
        } else if self.text_location.line_index > 0 {
            // 否则移动到上一行的结尾
            self.move_up(1);
            self.move_to_end_of_line();
        }
    }

    // 移动到当前行的开头
    fn move_to_start_of_line(&mut self) {
        self.text_location.grapheme_index = 0;
    }

    // 移动到当前行的结尾
    fn move_to_end_of_line(&mut self) {
        self.text_location.grapheme_index = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, Line::grapheme_count);
    }

    // 确保图形单元(列)索引有效，如果需要，将其调整到最左边的图形单元。
    // 不触发滚动。
    fn snap_to_valid_grapheme(&mut self) {
        self.text_location.grapheme_index = self
            .buffer
            .lines
            .get(self.text_location.line_index)
            .map_or(0, |line| {
                // 确保图形单元索引不超过当前行的最大图形单元索引
                min(line.grapheme_count(), self.text_location.grapheme_index)
            });
    }
    
    // 确保行索引有效，如果需要，将其调整到底部的行。
    // 不触发滚动。
    fn snap_to_valid_line(&mut self) {
        self.text_location.line_index = min(self.text_location.line_index, self.buffer.height());
    }

    // endregion
    // 文本位置移动代码结束

}

impl UIComponent for View {
    fn set_needs_redraw(&mut self, value: bool) {
        self.needs_redraw = value;
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    fn set_size(&mut self, size: Size) {
        self.size = size;
        self.scroll_text_location_into_view();
    }

    fn draw(&mut self, origin_y: usize) -> Result<(), Error> {
        let Size { height, width } = self.size;
        let end_y = origin_y.saturating_add(height);

        #[allow(clippy::integer_division)]
        // 计算垂直居中的位置，用于显示欢迎信息
        // 它可以稍微偏上一点或偏下一点，因为我们不在乎欢迎信息是否恰好位于正中间。
        let top_third = height / 3;
        // 获取滚动偏移量
        let scroll_top = self.scroll_offset.row;
        for current_row in origin_y..end_y {
            // 从终端上的当前行、原点和滚动偏移量计算缓冲区中的正确行。
            // 为了获得正确的行索引，我们必须取 current_row（屏幕上绝对的行位置）,
            // 减去 origin_y 以得到相对于视图的当前行（范围从 0 到 self.size.height）,
            // 然后加上滚动偏移量。
            let line_idx = current_row
                .saturating_sub(origin_y)
                .saturating_add(scroll_top);
            // 判断输出
            if let Some(line) = self.buffer.lines.get(line_idx) {
                let left = self.scroll_offset.col;
                let right = self.scroll_offset.col.saturating_add(width);
                Self::render_line(current_row, &line.get_visible_graphemes(left..right))?;
            } else if current_row == top_third && self.buffer.is_empty() {
                // 如果当前行是垂直居中的位置且缓冲区为空，显示欢迎信息
                Self::render_line(current_row, &Self::build_welcome_message(width))?;
            } else {
                // 否则，渲染波浪符 "~" 表示空白行
                Self::render_line(current_row, "~")?;
            }
        }
        Ok(())
    }
}