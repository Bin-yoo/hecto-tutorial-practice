use std::env;
use std::io::Error;
use std::panic::{set_hook, take_hook};
use crossterm::event::{read, Event, KeyEvent, KeyEventKind};
use editorcommand::EditorCommand;
use terminal::Terminal;
use view::View;

mod terminal;
mod view;
mod editorcommand;

pub struct Editor {
    should_quit: bool,
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

    // 判断按键事件
    fn evaluate_event(&mut self, event: Event) {
        // 判断是否应该处理该事件
        let should_process = match &event {
            Event::Key(KeyEvent { kind, .. }) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if should_process {
            match EditorCommand::try_from(event) {
                Ok(command) => {
                    // 判断退出
                    if matches!(command, EditorCommand::Quit) {
                        self.should_quit = true
                    } else {
                        self.view.handle_command(command);
                    }
                },
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        // panic!("无法处理命名: {err}");
                        // eprintln!("无法处理命令: {err}");
                    }
                }
            }
        } else {
            #[cfg(debug_assertions)]
            {
                // panic!("收到并丢弃了不支持的事件或非按键事件。");
                // eprintln!("收到并丢弃了不支持的事件或非按键事件: {:?}", event);
            }
        }
    }

    // 刷新屏幕
    fn refresh_screen(&mut self) {
        // 在刷新屏幕之前隐藏光标。
        let _ = Terminal::hide_caret();
        self.view.render();
        // 移动光标
        let _ = Terminal::move_caret_to(self.view.get_position());
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