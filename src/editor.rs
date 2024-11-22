use std::cmp::min;
use std::env;
use std::io::Error;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::event::Event::Key;
use terminal::{Position, Size, Terminal};
use view::View;

mod terminal;
mod view;

#[derive(Copy, Clone, Default)]
struct Location {
    x: usize,
    y: usize,
}

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    location: Location,
    view: View
}

impl Editor {
    pub fn run(&mut self) {
        Terminal::initialize().unwrap();
        // 处理命令行启动参数
        self.handle_args();
        // 启动 REPL 交互式循环
        let result = self.repl();
        // 终止终端
        Terminal::terminate().unwrap();
        // 处理 REPL 执行结果
        result.unwrap();
    }

    // 处理命令行启动参数
    fn handle_args(&mut self) {
        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            self.view.load(file_name);
        }
    }

    /// 监听和处理用户输入的事件。
    fn repl(&mut self) -> Result<(), Error> {
        loop {
            // 刷新屏幕
            self.refresh_screen()?;
            // 如果应该退出，跳出循环
            if self.should_quit {
                break;
            }
            // 读取用户输入事件
            let event = read()?;
            // 处理该事件
            self.evaluate_event(event)?;
        }
        Ok(())
    }

    // 移动光标
    fn move_point(&mut self, key_code: KeyCode) -> Result<(), Error> {
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

    // 判断按键事件
    fn evaluate_event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Key(KeyEvent {
                code, modifiers, kind: KeyEventKind::Press, ..
            }) => match (code, modifiers) {
                // 如果是 ctrl+q 就退出程序
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                },
                (
                    KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::PageDown
                    | KeyCode::PageUp
                    | KeyCode::End
                    | KeyCode::Home,
                    _
                ) => {
                    self.move_point(code)?;
                }
                _ => {},
            },
            Event::Resize(witdth_u16, height_u16) => {
                // 当终端大小发生变化时，调整视图大小
                // clippy::as_conversions: Will run into problems for rare edge case systems where usize < u16
                #[allow(clippy::as_conversions)]
                let height = height_u16 as usize;
                #[allow(clippy::as_conversions)]
                let width = witdth_u16 as usize;
                self.view.resize(
                    Size {
                        height,
                        width
                    }
                );
            },
            _ => {}
        }
        Ok(())
    }

    // 刷新屏幕
    fn refresh_screen(&mut self) -> Result<(), Error> {
        // 在刷新屏幕之前隐藏光标。
        Terminal::hide_caret()?;
        // 移动光标到初始位置
        Terminal::move_caret_to(Position::default())?;
        // 判断是否退出程序
        if self.should_quit {
            Terminal::clear_screen()?;
            Terminal::print("Goodbye.\r\n")?;
        } else {
            self.view.render()?;
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

}