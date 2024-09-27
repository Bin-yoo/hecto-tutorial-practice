use std::cmp::min;
use std::io::Error;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::event::KeyCode::Char;
use crossterm::event::Event::Key;
use terminal::{Position, Size, Terminal};

mod terminal;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Copy, Clone, Default)]
struct Location {
    x: usize,
    y: usize,
}

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    location: Location
}

impl Editor {
    pub fn run(&mut self) {
        Terminal::initialize().unwrap();
        let result = self.repl();
        Terminal::terminate().unwrap();
        result.unwrap();
    }

    // 交互
    fn repl(&mut self) -> Result<(), Error> {
        loop {
            self.refresh_screen()?;
            if self.should_quit {
                break;
            }
            let event = read()?;
            self.evaluate_event(&event)?;
        }
        Ok(())
    }

    // 判断按键事件
    fn evaluate_event(&mut self, event: &Event) -> Result<(), Error>{
        if let Key(KeyEvent {
            code, modifiers, kind: KeyEventKind::Press, ..
        }) = event
        {
            match code {
                // 如果是 ctrl+c 就退出程序
                Char('q') if *modifiers == KeyModifiers::CONTROL => {
                    self.should_quit = true;
                },
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageDown
                | KeyCode::PageUp
                | KeyCode::End
                | KeyCode::Home => {
                    self.move_point(*code)?;
                }
                _ => (),
            }
        }
        Ok(())
    }

    // 移动光标
    fn move_point(&mut self, key_code: KeyCode) -> Result<(), Error>{
        let Location { mut x, mut y } = self.location;
        let Size { height, width } = Terminal::size()?;
        // 计算x,y坐标
        match key_code {
            KeyCode::Up => y = y.saturating_sub(1),
            KeyCode::Down => y = min(height.saturating_sub(1), y.saturating_add(1)),
            KeyCode::Left => x = x.saturating_sub(1),
            KeyCode::Right => x = min(width.saturating_sub(1), x.saturating_add(1)),
            KeyCode::PageDown => y = height.saturating_sub(1),
            KeyCode::PageUp => y = 0,
            KeyCode::End => x = width.saturating_sub(1),
            KeyCode::Home => x = 0,
            _ => ()
        }

        // 将移动后的坐标保存
        self.location = Location { x, y };
        Ok(())
    }

    // 刷新屏幕
    fn refresh_screen(&self) -> Result<(), Error> {
        // 在刷新屏幕之前隐藏光标。
        Terminal::hide_caret()?;
        // 判断是否退出程序
        if self.should_quit {
            Terminal::clear_screen()?;
            Terminal::print("Goodbye.\r\n")?;
        } else {
            Self::draw_rows()?;
            // 根据x,y值移动光标
            Terminal::move_caret_to(
                Position {
                    col: self.location.x,
                    row: self.location.y
                }
            )?;
        }
        // 完成刷新后显示光标。
        Terminal::show_caret()?;
        // 输出缓冲区内容
        Terminal::execute()?;
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
        Terminal::print(welcome_message)?;
        Ok(())
    }

    // 打印空行
    fn draw_empty_row() -> Result<(), Error> {
        Terminal::print("~")?;
        Ok(())
    }

    // 绘制行
    fn draw_rows() -> Result<(), Error> {
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

}