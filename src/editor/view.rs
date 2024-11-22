use std::str;
use super::{editorcommand::{Direction, EditorCommand}, terminal::{Position, Size, Terminal}};
use buffer::Buffer;
use location::Location;

mod buffer;
mod location;
mod line;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct View {
    // 存储文本内容的缓冲区
    buffer: Buffer,
    // 标记是否需要重新渲染
    needs_redraw: bool,
    // 当前终端窗口大小
    size: Size,
    location: Location,
    scroll_offset: Location,
}

impl View {
    /// `resize` 方法用于调整 `View` 的尺寸，并设置标志要求重新渲染。
    ///
    /// # 参数
    /// - `to`: 新的终端窗口大小。
    pub fn resize(&mut self, to: Size) {
        self.size = to;
        self.scroll_location_into_view();
        // 设置成需要重新渲染
        self.needs_redraw = true;
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
        let top = self.scroll_offset.y;
        for current_row in 0..height {
            // 判断输出
            if let Some(line) = self.buffer.lines.get(current_row.saturating_add(top)) {
                let left = self.scroll_offset.x;
                let right = self.scroll_offset.x.saturating_add(width);
                Self::render_line(current_row, &line.get(left..right));
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

    /// 处理编辑器命令。
    ///
    /// # 参数
    /// - `command`: 编辑器命令。
    pub fn handle_command(&mut self, comand: EditorCommand) {
        match comand {
            EditorCommand::Resize(size) => self.resize(size),
            EditorCommand::Move(direction) => self.move_text_localtion(&direction),
            EditorCommand::Quit => {},
        }
    }

    /// 获取当前光标的物理位置。
    pub fn get_position(&self) -> Position {
        self.location.subtract(&self.scroll_offset).into()
    }

    /// 移动文本光标位置。
    ///
    /// # 参数
    /// - `direction`: 移动方向。
    fn move_text_localtion(&mut self, direction: &Direction) {
        let Location { mut x, mut y } = self.location;
        let Size { height, width } = self.size;
        match direction {
            Direction::Up => {
                y = y.saturating_sub(1);
            }
            Direction::Down => {
                y = y.saturating_add(1);
            }
            Direction::Left => {
                x = x.saturating_sub(1);
            }
            Direction::Right => {
                x = x.saturating_add(1);
            }
            Direction::PageUp => {
                y = 0;
            }
            Direction::PageDown => {
                y = height.saturating_sub(1);
            }
            Direction::Home => {
                x = 0;
            }
            Direction::End => {
                x = width.saturating_sub(1);
            }
        }

        self.location = Location { x, y };
        self.scroll_location_into_view();
    }

    fn scroll_location_into_view(&mut self) {
        let Location { x, y } = self.location;
        let Size { width, height } = self.size;
        let mut offset_changed = false;

        // 垂直滚动
        if y < self.scroll_offset.y {
            // 如果光标的纵坐标 y 小于当前滚动偏移量 self.scroll_offset.y，
            // 这意味着光标已经在可视区域的顶部之上。
            // 因此，将 self.scroll_offset.y 设置为 y，使光标回到可视区域的顶部。
            self.scroll_offset.y = y;
            offset_changed = true;
        } else if y >= self.scroll_offset.y.saturating_add(height) {
            // 如果光标的纵坐标 y 大于等于当前滚动偏移量加上终端高度 self.scroll_offset.y.saturating_add(height)，
            // 这意味着光标已经在可视区域的底部之下。
            // 因此，将 self.scroll_offset.y 设置为 y.saturating_sub(height).saturating_add(1)，
            // 使光标回到可视区域的底部。
            self.scroll_offset.y = y.saturating_sub(height).saturating_add(1);
            offset_changed = true;
        }

        // 水平滚动
        if x < self.scroll_offset.x {
            // 如果光标的横坐标 x 小于当前滚动偏移量 self.scroll_offset.x，
            // 这意味着光标已经在可视区域的左侧之上。
            // 因此，将 self.scroll_offset.x 设置为 x，使光标回到可视区域的左侧。
            self.scroll_offset.x = x;
            offset_changed = true;
        } else if x >= self.scroll_offset.x.saturating_add(width) {
            // 如果光标的横坐标 x 大于等于当前滚动偏移量加上终端宽度 self.scroll_offset.x.saturating_add(width)，
            // 这意味着光标已经在可视区域的右侧之外。
            // 因此，将 self.scroll_offset.x 设置为 x.saturating_sub(width).saturating_add(1)，
            // 使光标回到可视区域的右侧。
            self.scroll_offset.x = x.saturating_sub(width).saturating_add(1);
            offset_changed = true;
        }

        self.needs_redraw = offset_changed
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
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            needs_redraw: true,
            // 尝试获取终端的当前大小，如果失败则使用默认值
            size: Terminal::size().unwrap_or_default(),
            location: Location::default(),
            scroll_offset: Location::default()
        }
    }
}