use std::{io::Error, str};
use super::terminal::{Size, Terminal};

mod buffer;
use buffer::Buffer;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct View {
    buffer: Buffer
}

impl View {
    // 渲染欢迎语
    pub fn render_welcome_screen() -> Result<(), Error> {
        let Size { height, .. } = Terminal::size()?;
        for current_row in 0..height {
            // 清除当前行
            Terminal::clear_line()?;
            // 我们不介意欢迎消息是否被精确地放在中间,允许它稍微偏上或偏下一点。
            #[allow(clippy::integer_division)]
            if current_row == height / 3 {
                Self::draw_welcome_message()?;
            } else {
                // 输出空行
                Self::draw_empty_row()?;
            }
            // 当前行小于高度就输出回车符和换行符
            if current_row.saturating_add(1) < height {
                Terminal::print("\r\n")?;
            }
        }
        Ok(())
    }

    // 渲染缓冲区内容
    pub fn render_buffer(&self) -> Result<(), Error> {
        let Size { height, .. } = Terminal::size()?;

        for current_row in 0..height {
            Terminal::clear_line()?;
            // 判断输出
            if let Some(line) = self.buffer.lines.get(current_row) {
                Terminal::print(line)?;
                Terminal::print("\r\n")?;
            } else {
                Self::draw_empty_row()?;
            }
        }
        Ok(())
    }

    // 渲染
    pub fn render(&self) -> Result<(), Error> {
        if self.buffer.is_empty() {
            Self::render_welcome_screen()?;
        } else {
            self.render_buffer()?;
        }
        Ok(())
    }

    // 打印欢迎语
    fn draw_welcome_message() -> Result<(), Error> {
        let mut welcome_message = format!("{NAME} editor -- version {VERSION}");
        let width = Terminal::size()?.width;
        let len = welcome_message.len();
        // 终端宽度减去欢迎语长度得到空余部分长度,再除以2
        // we allow this since we don't care if our welcome message is put _exactly_ in the middle.
        // it's allowed to be a bit to the left or right.
        #[allow(clippy::integer_division)]
        let padding = (width.saturating_sub(len)) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        // 留白格式化整行输出内容
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        Terminal::print(&welcome_message)?;
        Ok(())
    }

    // 打印空行
    fn draw_empty_row() -> Result<(), Error> {
        Terminal::print("~")?;
        Ok(())
    }

    // 读取文件
    pub fn load(&mut self, file_name: &str) {
        if let Ok(buffer) = Buffer::load(file_name) {
            self.buffer = buffer
        }
    }
}