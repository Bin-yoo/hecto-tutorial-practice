use std::env;
use std::io::Error;
use std::panic::{set_hook, take_hook};
use crossterm::event::{read, Event, KeyEvent, KeyEventKind};
use editorcommand::EditorCommand;
use statusbar::StatusBar;
use terminal::Terminal;
use view::View;

mod terminal;
mod view;
mod editorcommand;
mod statusbar;

#[derive(Default, Eq, PartialEq, Debug)]
pub struct DocumentStatus {
    // 文档总行数
    total_lines: usize,
    // 当前行
    current_line_index: usize,
    // 是否已修改
    is_modified: bool,
    // 文件名
    file_name: Option<String>,
}

pub struct Editor {
    should_quit: bool,
    view: View,
    status_bar: StatusBar
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
        // 创建视图组件,空出底部两行
        let mut view = View::new(2);
        // 处理命令行参数，尝试加载文件
        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            view.load(file_name);
        }

        Ok(Self {
            should_quit: false,
            view,
            // 空出一行
            status_bar: StatusBar::new(1),
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
            // 更新状态栏
            let status = self.view.get_status();
            self.status_bar.update_status(status);
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
            if let Ok(command) = EditorCommand::try_from(event) {
                if matches!(command, EditorCommand::Quit) {
                    self.should_quit = true;
                } else {
                    self.view.handle_command(command);
                    if let EditorCommand::Resize(size) = command {
                        self.status_bar.resize(size);
                    }
                }
            }
        }
    }

    // 刷新屏幕
    fn refresh_screen(&mut self) {
        // 在刷新屏幕之前隐藏光标。
        let _ = Terminal::hide_caret();
        // 渲染view
        self.view.render();
        // 渲染状态栏
        self.status_bar.render();
        // 移动光标
        let _ = Terminal::move_caret_to(self.view.caret_position());
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