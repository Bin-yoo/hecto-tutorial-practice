use std::env;
use std::io::Error;
use std::panic::{set_hook, take_hook};
use crossterm::event::{read, Event, KeyEvent, KeyEventKind};

use self::command::{
Command::{self, Edit, Move, System},
    Edit::InsertNewline,
    System::{Dismiss, Quit, Resize, Save}
};

use commandbar::CommandBar;
use messagebar::MessageBar;
use statusbar::StatusBar;
use terminal::Terminal;
use uicomponent::UIComponent;
use view::View;
use position::Position;
use size::Size;
use line::Line;
use documentstatus::DocumentStatus;

mod terminal;
mod view;
mod command;
mod statusbar;
mod messagebar;
mod uicomponent;
mod documentstatus;
mod line;
mod commandbar;
mod position;
mod size;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// 为保持时进行退出操作所需操作次数
const QUIT_TIMES: u8 = 3;

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    view: View,
    // 状态栏
    status_bar: StatusBar,
    // 消息栏
    message_bar: MessageBar,
    // 命令栏
    command_bar: Option<CommandBar>,
    terminal_size: Size,
    title: String,
    // 用于跟踪用户尝试退出的次数
    quit_times: u8,
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

        // 初始化编辑器参数
        let mut editor = Self::default();
        let size = Terminal::size().unwrap_or_default();
        editor.resize(size);

        // 设置编辑器默认消息栏消息
        editor
            .message_bar
            .update_message("HELP: Ctrl-S = save | Ctrl-Q = quit");

        // 处理命令行参数，尝试加载文件
        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            if editor.view.load(file_name).is_err() {
                editor
                    .message_bar
                    .update_message(&format!("ERR: Could not open file: {file_name}"));
            }
        }

        // 刷新状态
        editor.refresh_status();

        Ok(editor)
    }

    /// 调整编辑器大小
    fn resize(&mut self, size: Size) {
        self.terminal_size = size;
        // 空出底部两行给消息栏和状态栏
        self.view.resize(Size {
            height: size.height.saturating_sub(2),
            width: size.width,
        });
        self.message_bar.resize(Size {
            height: 1,
            width: size.width,
        });
        self.status_bar.resize(Size {
            height: 1,
            width: size.width,
        });

        if let Some(command_bar) = &mut self.command_bar {
            command_bar.resize(Size {
                height: 1,
                width: size.width,
            });
        }
    }

    /// 刷新编辑器状态
    pub fn refresh_status(&mut self) {
        // 获取状态,格式化title输出
        let status = self.view.get_status();
        let title = format!("{} - {NAME}", status.file_name);
        // 更新状态栏
        self.status_bar.update_status(status);
        // 判断标题是否已更改,并且写入终端成功.则更新editor保存的title
        if title != self.title && matches!(Terminal::set_title(&title), Ok(())) {
            self.title = title;
        }
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
            if let Ok(command) = Command::try_from(event) {
                self.process_command(command);
            }
        }
    }

    /// 处理命令
    fn process_command(&mut self, command: Command) {
        match command {
            System(Quit) => {
                if self.command_bar.is_none() {
                    self.handle_quit();
                }
            },
            System(Resize(size)) => self.resize(size),
            // 在进行其他操作后重置累计的退出操作计数
            _ => self.reset_quit_times(), 
        }
        match command {
            // already handled above
            System(Quit | Resize(_)) => {}
            System(Save) => {
                if self.command_bar.is_none() {
                    self.handle_save();
                }
            },
            System(Dismiss) => {
                // 如果存在命令栏（即正处于提示符内）,我们将通过显示 '保存已取消' 的消息来关闭它。
                if self.command_bar.is_some() {
                    self.dismiss_prompt();
                    self.message_bar.update_message("Save aborted.");
                }
            },
            Edit(edit_command) => {
                // 检查是否有一个活动的命令栏,否则让view来处理编辑的命令
                if let Some(command_bar) = &mut self.command_bar {
                    // 如果编辑命令是插入新行(对应操作是Enter回车键),则获取命令栏中的值作为文件名进行保存
                    // 否则让命令栏来处理编辑的命令
                    if matches!(edit_command, InsertNewline) {
                        let file_name = command_bar.value();
                        self.dismiss_prompt();
                        self.save(Some(&file_name));
                    } else {
                        command_bar.handle_edit_command(edit_command);
                    }
                } else {
                    self.view.handle_edit_command(edit_command);
                }
            },
            Move(move_command) => {
                if self.command_bar.is_none() {
                    self.view.handle_move_command(move_command);
                }
            }
        }
    }

    /// 关闭(命令栏)提示
    fn dismiss_prompt(&mut self) {
        self.command_bar = None;
        // 确保消息栏重绘
        self.message_bar.set_needs_redraw(true);
    }

    /// 显示(命令栏)提示
    fn show_prompt(&mut self) {
        // 创建新的 CommandBar
        let mut command_bar = CommandBar::default();
        // 设置文本提示和尺寸
        command_bar.set_prompt("Save as: ");
        command_bar.resize(Size {
            height: 1,
            width: self.terminal_size.width,
        });
        // 设置需要重绘
        command_bar.set_needs_redraw(true);
        self.command_bar = Some(command_bar);
    }

    /// 处理文件保存
    fn handle_save(&mut self) {
        if self.view.is_file_loaded() {
            self.save(None);
        } else {
            self.show_prompt();
        }
    }

    /// 文件保存
    fn save(&mut self, file_name: Option<&str>) {
        let result = if let Some(name) = file_name {
            self.view.save_as(name)
        } else {
            self.view.save()
        };
        if result.is_ok() {
            self.message_bar.update_message("File saved successfully.");
        } else {
            self.message_bar.update_message("Error writing file!");
        }
    }

    /// 处理退出编辑器
    // clippy::arithmetic_side_effects: quit_times is guaranteed to be between 0 and QUIT_TIMES
    #[allow(clippy::arithmetic_side_effects)]
    fn handle_quit(&mut self) {
        // 未进行修改或退出操作次数累计达到3次,则设置退出标识为true
        if !self.view.get_status().is_modified || self.quit_times + 1 == QUIT_TIMES {
            self.should_quit = true;
        } else if self.view.get_status().is_modified {
            // 文件已进行修改,则格式化消息提示更新到消息栏,并累计退出操作次数
            self.message_bar.update_message(&format!(
                "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                QUIT_TIMES - self.quit_times - 1
            ));
            self.quit_times += 1;
        }
    }

    /// 重查退出操作次数
    fn reset_quit_times(&mut self) {
        if self.quit_times > 0 {
            self.quit_times = 0;
            self.message_bar.update_message("");
        }
    }

    // 刷新屏幕
    fn refresh_screen(&mut self) {
        if self.terminal_size.height == 0 || self.terminal_size.width == 0 {
            return;
        }
        // 底部栏位所占高度
        let bottom_bar_row = self.terminal_size.height.saturating_sub(1);
        // 在刷新屏幕之前隐藏光标。
        let _ = Terminal::hide_caret();
        // 判断是渲染命令栏还是消息栏
        if let Some(command_bar) = &mut self.command_bar {
            command_bar.render(bottom_bar_row);
        } else {
            self.message_bar.render(bottom_bar_row);
        }
        // 渲染状态栏
        if self.terminal_size.height > 1 {
            self.status_bar.render(self.terminal_size.height.saturating_sub(2));
        }
        // 渲染view
        if self.terminal_size.height > 2 {
            self.view.render(0);
        }
        // 判断是从命令栏还是view获取光标位置
        let new_caret_pos = if let Some(command_bar) = &self.command_bar {
            Position {
                row: bottom_bar_row,
                col: command_bar.caret_position_col()
            }
        } else {
            self.view.caret_position()
        };
        // 移动光标
        let _ = Terminal::move_caret_to(new_caret_pos);
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