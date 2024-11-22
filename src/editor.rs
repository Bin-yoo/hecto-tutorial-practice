use std::cmp::min;
use std::env;
use std::io::Error;
use std::panic::{set_hook, take_hook};
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

pub struct Editor {
    should_quit: bool,
    location: Location,
    view: View
}

impl Editor {

    /// 创建一个新的 `Editor` 实例。
    pub fn new() -> Result<Self, Error> {
        // 捕获并处理程序崩溃，确保终端能够正确恢复
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        // 初始化终端
        Terminal::initialize()?;
        // 创建默认的视图组件
        let mut view = View::default();
        // 处理命令行参数，尝试加载文件
        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            view.load(file_name);
        }

        Ok(Self {
            should_quit: false,
            location: Location::default(),
            view
        })
    }

    /// 运行编辑器主循环。
    pub fn run(&mut self) {
        loop {
            // 刷新屏幕
            self.refresh_screen();
            // 如果应该退出，跳出循环
            if self.should_quit {
                break;
            }
            // 读取用户输入事件
            match read() {
                Ok(event) => self.evaluate_event(event),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("无法读取事件: {err:?}");
                    }
                }
            }
        }
    }

    // 移动光标
    fn move_point(&mut self, key_code: KeyCode) {
        let Location { mut x, mut y } = self.location;
        let Size { height, width } = Terminal::size().unwrap_or_default();
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
    }

    // 判断按键事件
    fn evaluate_event(&mut self, event: Event) {
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
                    self.move_point(code);
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
    }

    // 刷新屏幕
    fn refresh_screen(&mut self) {
        // 在刷新屏幕之前隐藏光标。
        let _ = Terminal::hide_caret();
        self.view.render();
        // 移动光标
        let _ = Terminal::move_caret_to(Position {
            col: self.location.x,
            row: self.location.y
        });
        // 完成刷新后显示光标。
        let _ = Terminal::show_caret();
        // 输出缓冲区内容
        let _ = Terminal::execute();
    }

}

impl Drop for Editor {
    /// 在 `Editor` 被销毁时调用，确保终端恢复正常状态。
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("Goodbye.\r\n");
        }
    }
}