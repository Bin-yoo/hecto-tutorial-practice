use std::{cmp::min, str};
use super::{editorcommand::{Direction, EditorCommand}, terminal::{Position, Size, Terminal}};
use buffer::Buffer;
use line::Line;

mod buffer;
mod line;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Copy, Clone, Default)]
pub struct Location {
    pub grapheme_index: usize,
    pub line_index: usize,
}

pub struct View {
    // 存储文本内容的缓冲区
    buffer: Buffer,
    // 标记是否需要重新渲染
    needs_redraw: bool,
    // 当前终端窗口大小
    size: Size,
    // 文档中位置
    text_location: Location,
    // view的偏移
    scroll_offset: Position,
}

impl View {
    /// 处理编辑器命令。
    ///
    /// # 参数
    /// - `command`: 编辑器命令。
    pub fn handle_command(&mut self, comand: EditorCommand) {
        match comand {
            EditorCommand::Resize(size) => self.resize(size),
            EditorCommand::Move(direction) => self.move_text_location(&direction),
            EditorCommand::Quit => {},
        }
    }

    /// 读取文件内容并加载到缓冲区。
    ///
    /// # 参数
    /// - `file_name`: 要加载的文件名。
    ///
    /// 如果文件加载成功，则将其内容保存到缓冲区，并标记视图需要重新渲染。
    pub fn load(&mut self, file_name: &str) {
        if let Ok(buffer) = Buffer::load(file_name) {
            self.buffer = buffer;
            self.needs_redraw = true;
        }
    }

    /// `resize` 方法用于调整 `View` 的尺寸，并设置标志要求重新渲染。
    ///
    /// # 参数
    /// - `to`: 新的终端窗口大小。
    pub fn resize(&mut self, to: Size) {
        self.size = to;
        self.scroll_text_location_into_view();
        // 设置成需要重新渲染
        self.needs_redraw = true;
    }

    // region: Rendering
    // 渲染方法代码


    /// 渲染整个视图内容。
    ///
    /// 如果视图的尺寸发生了变化，或内容发生了变化，就会重新渲染。
    ///
    /// 渲染逻辑如下：
    /// - 如果不需要重新渲染，直接返回。
    /// - 检查终端窗口的大小，如果大小为 0，跳过渲染。
    /// - 否则，逐行渲染内容。
    pub fn render(&mut self) {
        // 不需重新渲染则直接返回
        if !self.needs_redraw {
            return;
        }
         // 如果终端窗口的高度或宽度为 0，跳过渲染
        let Size{ height, width } = self.size;
        if height == 0 || width == 0 {
            return;
        }

        #[allow(clippy::integer_division)]
        // 计算垂直居中的位置，用于显示欢迎信息
        // 它可以稍微偏上一点或偏下一点，因为我们不在乎欢迎信息是否恰好位于正中间。
        let vertical_center = height / 3;
        // 获取滚动偏移量
        let top = self.scroll_offset.row;
        for current_row in 0..height {
            // 判断输出
            if let Some(line) = self.buffer.lines.get(current_row.saturating_add(top)) {
                let left = self.scroll_offset.col;
                let right = self.scroll_offset.col.saturating_add(width);
                Self::render_line(current_row, &line.get_visible_graphemes(left..right));
            } else if current_row == vertical_center && self.buffer.is_empty() {
                // 如果当前行是垂直居中的位置且缓冲区为空，显示欢迎信息
                Self::render_line(current_row, &Self::build_welcome_message(width));
            } else {
                // 否则，渲染波浪符 "~" 表示空白行
                Self::render_line(current_row, "~");
            }
        }

        // 渲染完毕，标记不再需要重新渲染
        self.needs_redraw = false;
    }

    /// 渲染指定行的内容。
    ///
    /// # 参数
    /// - `at`: 行号，表示要渲染到的目标行。
    /// - `line_text`: 要渲染的文本内容。
    ///
    /// 清除指定行的内容，将文本渲染到该终端行。
    fn render_line(at: usize, line_text: &str) {
        // 打印传入的行内容
        let result = Terminal::print_row(at, line_text);
        // 断言输出操作是否成功，如果失败则会在debug模式下中断程序执行
        debug_assert!(result.is_ok(), "渲染行失败");
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
            return " ".to_string();
        }
        let welcome_message = format!("{NAME} editor -- version {VERSION}");
        let len = welcome_message.len();
        // 宽度不够就返回波浪符
        if width <= len {
            return "~".to_string();
        }
        // 终端宽度减去欢迎语长度得到空余部分长度,再除以2
        // 计算欢迎信息两侧的空白填充长度，确保其居中
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len).saturating_sub(1)) / 2;
        // 构造完整的欢迎信息字符串，并进行截断以适应终端宽度
        let mut full_message = format!("~{}{}", " ".repeat(padding), welcome_message);
        full_message.truncate(width);
        full_message
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
        self.needs_redraw = self.needs_redraw || offset_changed;
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
        
        self.needs_redraw = self.needs_redraw || offset_changed;
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

    // 移动文本位置
    fn move_text_location(&mut self, direction: &Direction) {
        let Size { height, .. } = self.size;
        // 这个匹配语句会移动位置，但不检查边界。
        // 最终的边界检查在匹配语句之后进行。
        match direction {
            Direction::Up => self.move_up(1),
            Direction::Down => self.move_down(1),
            Direction::Left => self.move_left(),
            Direction::Right => self.move_right(),
            Direction::PageUp => self.move_up(height.saturating_sub(1)),
            Direction::PageDown => self.move_down(height.saturating_sub(1)),
            Direction::Home => self.move_to_start_of_line(),
            Direction::End => self.move_to_end_of_line(),
        }

        // 滚动视图以使文本位置可见
        self.scroll_text_location_into_view();
    }

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
        } else {
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

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            // 尝试获取终端的当前大小，如果失败则使用默认值
            size: Terminal::size().unwrap_or_default(),
            text_location: Location::default(),
            scroll_offset: Position::default()
        }
    }
}