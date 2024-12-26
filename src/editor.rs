use std::env;
use std::io::Error;
use std::panic::{set_hook, take_hook};
use crossterm::event::{read, Event, KeyEvent, KeyEventKind};

use command::{
    Command::{self, Edit, Move, System},
    Edit::InsertNewline,
    Move::{Down, Left, Right, Up},
    System::{Dismiss, Quit, Resize, Save, Search}
};

use terminal::Terminal;
use uicomponents::{CommandBar,MessageBar,View, StatusBar, UIComponent};
use position::{Col, Position, Row};
use size::Size;
use line::Line;
use documentstatus::DocumentStatus;
use annotatedstring::{AnnotatedString, AnnotationType};

mod annotatedstring;
mod terminal;
mod command;
mod uicomponents;
mod documentstatus;
mod line;
mod position;
mod size;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// 为保持时进行退出操作所需操作次数
const QUIT_TIMES: u8 = 3;

/// 提示类型枚举
#[derive(Eq, PartialEq, Default)]
enum PromptType {
    Search,
    Save,
    #[default]
    None,
}

impl PromptType {
    fn is_none(&self) -> bool {
        *self == Self::None
    }
}

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    view: View,
    // 状态栏
    status_bar: StatusBar,
    // 消息栏
    message_bar: MessageBar,
    // 命令栏
    command_bar: CommandBar,
    // 提示类型
    prompt_type: PromptType,
    // 终端大小
    terminal_size: Size,
    title: String,
    // 用于跟踪用户尝试退出的次数
    quit_times: u8,
}

impl Editor {
    // region: struct lifecycle

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

        // 处理大小
        editor.handle_resize_command(size);
        // 设置编辑器默认消息栏消息
        editor.update_message("HELP: Ctrl-F = find | Ctrl-S = save | Ctrl-Q = quit");

        // 处理命令行参数，尝试加载文件
        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            debug_assert!(!file_name.is_empty());
            if editor.view.load(file_name).is_err() {
                editor.update_message(&format!("ERR: Could not open file: {file_name}"));
            }
        }

        // 刷新状态
        editor.refresh_status();

        Ok(editor)
    }

    // endregion

    // region: Event Loop

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
                    #[cfg(not(debug_assertions))]
                    {
                        let _ = err;
                    }
                }
            }
            // 刷新状态
            self.refresh_status();
        }
    }

    /// 刷新屏幕
    fn refresh_screen(&mut self) {
        if self.terminal_size.height == 0 || self.terminal_size.width == 0 {
            return;
        }
        // 底部栏位所占高度
        let bottom_bar_row = self.terminal_size.height.saturating_sub(1);
        // 在刷新屏幕之前隐藏光标。
        let _ = Terminal::hide_caret();
        // 判断是渲染命令栏还是消息栏
        if self.in_prompt() {
            self.command_bar.render(bottom_bar_row);
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
        let new_caret_pos = if self.in_prompt() {
            Position {
                row: bottom_bar_row,
                col: self.command_bar.caret_position_col()
            }
        } else {
            self.view.caret_position()
        };
        debug_assert!(new_caret_pos.col <= self.terminal_size.width);
        debug_assert!(new_caret_pos.row <= self.terminal_size.height);

        // 移动光标
        let _ = Terminal::move_caret_to(new_caret_pos);
        // 完成刷新后显示光标。
        let _ = Terminal::show_caret();
        // 输出缓冲区内容
        let _ = Terminal::execute();
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

    /// 判断按键事件
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

    // endregion

    // region command handling

    /// 处理命令
    fn process_command(&mut self, command: Command) {
        if let System(Resize(size)) = command {
            self.handle_resize_command(size);
            return;
        }
        match self.prompt_type {
            PromptType::Search => self.process_command_during_search(command),
            PromptType::Save => self.process_command_during_save(command),
            PromptType::None => self.process_command_no_prompt(command),
        }
    }

    /// 无提示时处理命令
    fn process_command_no_prompt(&mut self, command: Command) {
        // 处理退出
        if matches!(command, System(Quit)) {
            self.handle_quit_command();
            return;
        }
        // 其他操作就重置退出操作累计次数
        self.reset_quit_times();

        match command {
            // 忽略退出和调整大小
            System(Quit | Resize(_) | Dismiss) => {}
            // 搜索:设置提示
            System(Search) => self.set_prompt(PromptType::Search),
            // 保存
            System(Save) => self.handle_save_command(),
            // 编辑
            Edit(edit_command) => self.view.handle_edit_command(edit_command),
            // 移动光标
            Move(move_command) => self.view.handle_move_command(move_command),
        }
    }

    // endregion

    // region resize command handling

    /// 处理调整大小的命令
    fn handle_resize_command(&mut self, size: Size) {
        self.terminal_size = size;
        // 空出底部两行给消息栏和状态栏
        self.view.resize(Size {
            height: size.height.saturating_sub(2),
            width: size.width,
        });
        let bar_size = Size {
            height: 1,
            width: size.width,
        };
        self.message_bar.resize(bar_size);
        self.status_bar.resize(bar_size);
        self.command_bar.resize(bar_size);
    }

    // endregion

    // region quit command handling

    /// 处理退出编辑器命令
    // clippy::arithmetic_side_effects: quit_times is guaranteed to be between 0 and QUIT_TIMES
    #[allow(clippy::arithmetic_side_effects)]
    fn handle_quit_command(&mut self) {
        // 未进行修改或退出操作次数累计达到3次,则设置退出标识为true
        if !self.view.get_status().is_modified || self.quit_times + 1 == QUIT_TIMES {
            self.should_quit = true;
        } else if self.view.get_status().is_modified {
            // 文件已进行修改,则格式化消息提示更新到消息栏,并累计退出操作次数
            self.update_message(&format!(
                "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                QUIT_TIMES - self.quit_times - 1
            ));
            self.quit_times += 1;
        }
    }

    /// 重置退出操作次数
    fn reset_quit_times(&mut self) {
        if self.quit_times > 0 {
            self.quit_times = 0;
            self.update_message("");
        }
    }

    // endregion

    // region save command & prompt handling

    /// 处理文件保存
    fn handle_save_command(&mut self) {
        if self.view.is_file_loaded() {
            self.save(None);
        } else {
            self.set_prompt(PromptType::Save);
        }
    }

    /// 处理保存时的命令
    fn process_command_during_save(&mut self, command: Command) {
        match command {
            // 忽略无关的操作
            System(Quit | Resize(_) | Search | Save) | Move(_) => {}
            // 丢弃保存操作
            System(Dismiss) => {
                self.set_prompt(PromptType::None);
                self.update_message("Save aborted.");
            }
            // 按enter确认保存
            Edit(InsertNewline) => {
                let file_name = self.command_bar.value();
                self.save(Some(&file_name));
                self.set_prompt(PromptType::None);
            }
            // 命令栏输入
            Edit(edit_command) => self.command_bar.handle_edit_command(edit_command),
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
            self.update_message("File saved successfully.");
        } else {
            self.update_message("Error writing file!");
        }
    }

    // endregion

    // region search command & prompt handling
    
    /// 处理搜索时的命令
    fn process_command_during_search(&mut self, command: Command) {
        match command {
            // 关闭搜索,回到搜索前的文本位置
            System(Dismiss) => {
                self.set_prompt(PromptType::None);
                self.view.dismiss_search();
            }
            // 按Enter时，调用exit_search保留缓冲区中的当前位置。
            Edit(InsertNewline) => {
                self.set_prompt(PromptType::None);
                self.view.exit_search();
            }
            // 在命令行输入要搜索的内容,调用搜索
            Edit(edit_command) => {
                self.command_bar.handle_edit_command(edit_command);
                let query = self.command_bar.value();
                self.view.search(&query);
            }
            // 在搜索状态上下左右进行切换已识别的搜索内容
            Move(Right | Down) => self.view.search_next(),
            Move(Up | Left) => self.view.search_prev(),
            // 忽略无关的操作
            System(Quit | Resize(_) | Search | Save) | Move(_) => {}
        }
    }

    // endregion

    // region message & command bar
    
    /// 设置消息栏信息
    fn update_message(&mut self, new_message: &str) {
        self.message_bar.update_message(new_message);
    }

    // endregion


    //region prompt handling

    /// 获取是否有提示
    fn in_prompt(&self) -> bool {
        !self.prompt_type.is_none()
    }

    /// 设置提示
    fn set_prompt(&mut self, prompt_type: PromptType) {
        match prompt_type {
            //确保消息栏能在下一个循环周期重绘
            PromptType::None => self.message_bar.set_needs_redraw(true),
            // 保存提示
            PromptType::Save => self.command_bar.set_prompt("Save as: "),
            // 搜索提示
            PromptType::Search => {
                // 进入搜索
                self.view.enter_search();
                self.command_bar.set_prompt("Search (Esc to cancel, Arrows to navigate): ");
            }
        }
        self.command_bar.clear_value();
        self.prompt_type = prompt_type;
    }
    // end region

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