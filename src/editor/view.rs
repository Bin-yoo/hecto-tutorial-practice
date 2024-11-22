use std::{io::Error, str};
use super::terminal::{Position, Size, Terminal};

mod buffer;
use buffer::Buffer;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct View {
    // 存储文本内容的缓冲区
    buffer: Buffer,
    // 标记是否需要重新渲染
    needs_redraw: bool,
    // 当前终端窗口大小
    size: Size,
}

impl View {
    /// `resize` 方法用于调整 `View` 的尺寸，并设置标志要求重新渲染。
    ///
    /// # 参数
    /// - `to`: 新的终端窗口大小。
    pub fn resize(&mut self, to: Size) {
        self.size = to;
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
    fn render_line(at: usize, line_text: &str) -> Result<(), Error> {
        // 移动到对应行
        Terminal::move_caret_to(Position{ row: at, col: 0 })?;
        // 清除原本行内容
        Terminal::clear_line()?;
        // 打印传入的行内容
        Terminal::print(line_text)?;

        Ok(())
    }

    /// 渲染整个视图内容。
    ///
    /// 如果视图的尺寸发生了变化，或内容发生了变化，就会重新渲染。
    ///
    /// 渲染逻辑如下：
    /// - 如果不需要重新渲染，直接返回。
    /// - 检查终端窗口的大小，如果大小为 0，跳过渲染。
    /// - 否则，逐行渲染内容。
    pub fn render(&mut self) -> Result<(), Error> {
        // 不需重新渲染则直接返回
        if !self.needs_redraw {
            return Ok(())
        }
         // 如果终端窗口的高度或宽度为 0，跳过渲染
        let Size{ height, width } = self.size;
        if height == 0 || width == 0 {
            return Ok(())
        }

        #[allow(clippy::integer_division)]
        // 计算垂直居中的位置，用于显示欢迎信息
        // 它可以稍微偏上一点或偏下一点，因为我们不在乎欢迎信息是否恰好位于正中间。
        let vertical_center = height / 3;
        for current_row in 0..height {
            // 判断输出
            if let Some(line) = self.buffer.lines.get(current_row) {
                // 如果当前行的文本内容超过终端宽度，进行截断
                let truncate_line = if line.len() >= width {
                    &line[0..width]
                } else {
                    line
                };
                Self::render_line(current_row, truncate_line)?;
            } else if current_row == vertical_center && self.buffer.is_empty() {
                // 如果当前行是垂直居中的位置且缓冲区为空，显示欢迎信息
                Self::render_line(current_row, &Self::build_welcome_message(width))?;
            } else {
                // 否则，渲染波浪符 "~" 表示空白行
                Self::render_line(current_row, "~")?;
            }
        }

        // 渲染完毕，标记不再需要重新渲染
        self.needs_redraw = false;
        Ok(())
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
            size: Terminal::size().unwrap_or_default(),
        }
    }
}