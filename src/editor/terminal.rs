use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::{queue, Command};
use crossterm::style::{Attribute, Print};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use std::io::{stdout, Error, Write};

#[derive(Default, Copy, Clone)]
pub struct Size {
    pub height: usize,
    pub width: usize,
}
#[derive(Copy, Clone, Default)]
pub struct Position {
    pub col: usize,
    pub row: usize,
}

impl Position {
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self {
            row: self.row.saturating_sub(other.row),
            col: self.col.saturating_sub(other.col),
        }
    }
}

/// 表示终端。
/// 平台边缘情况处理：当 `usize` < `u16` 时：
/// 不管终端的实际大小如何，此表示最多只能覆盖 `usize::MAX` 或 `u16::MAX` 行/列，取较小值。
/// 每个返回的大小都会截断为 `min(usize::MAX, u16::MAX)`。
/// 如果尝试将光标设置到这些边界之外，也会被截断。
pub struct Terminal;

impl Terminal {
    // 结束程序
    pub fn terminate() -> Result<(), Error> {
        // 退出备用屏幕
        Self::leave_alternate_screen()?;
        // 重新启用换行
        Self::enable_line_wrap()?;
        // 显示光标
        Self::show_caret()?;
        // 刷新缓冲区
        Self::execute()?;
        // 禁用原始模式
        disable_raw_mode()?;
        Ok(())
    }

    /// 初始化终端，
    pub fn initialize() -> Result<(), Error> {
        // 进入原始模式并切换到备用屏幕。
        enable_raw_mode()?;
        Self::enter_alternate_screen()?;
        // 禁用换行
        Self::disable_line_wrap()?;
        // 清屏
        Self::clear_screen()?;
        // 刷新缓冲区
        Self::execute()?;
        Ok(())
    }

    /// 禁用换行
    pub fn disable_line_wrap() -> Result<(), Error> {
        Self::queue_command(DisableLineWrap)?;
        Ok(())
    }

    /// 启用换行
    pub fn enable_line_wrap() -> Result<(), Error> {
        Self::queue_command(EnableLineWrap)?;
        Ok(())
    }

    /// 设置终端标题
    pub fn set_title(title: &str) -> Result<(), Error> {
        Self::queue_command(SetTitle(title))?;
        Ok(())
    }

    /// 进入备用屏幕。
    pub fn enter_alternate_screen() -> Result<(), Error> {
        Self::queue_command(EnterAlternateScreen)?;
        Ok(())
    }

    /// 退出备用屏幕。
    pub fn leave_alternate_screen() -> Result<(), Error> {
        Self::queue_command(LeaveAlternateScreen)?;
        Ok(())
    }

    /// 清除终端屏幕
    pub fn clear_screen() -> Result<(), Error> {
        Self::queue_command(Clear(ClearType::All))?;
        Ok(())
    }

    /// 清除当前行
    pub fn clear_line() -> Result<(), Error> {
        Self::queue_command(Clear(ClearType::CurrentLine))?;
        Ok(())
    }

    /// 移动终端光标至指定位置
    /// # Arguments
    /// * `Position` - 要移动光标到的位置。如果坐标超过 `u16::MAX`，会被截断。
    pub fn move_caret_to(position: Position) -> Result<(), Error> {
        // clippy::as_conversions: See doc above
        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
        Self::queue_command(MoveTo(position.col as u16, position.row as u16))?;
        Ok(())
    }

    // 隐藏终端光标
    pub fn hide_caret() -> Result<(), Error> {
        Self::queue_command(Hide)?;
        Ok(())
    }

    // 显示终端光标
    pub fn show_caret() -> Result<(), Error> {
        Self::queue_command(Show)?;
        Ok(())
    }

    /// 在指定行打印文本
    pub fn print_row(row: usize, line_text: &str) -> Result<(), Error> {
        // 移动光标到指定行的开头
        Self::move_caret_to(Position { row, col: 0})?;
        // 清除当前行并打印
        Self::clear_line()?;
        Self::print(line_text)?;
        Ok(())
    }

    /// 在指定行打印颜色反转的文本
    pub fn print_inverted_row(row: usize, line_text: &str) -> Result<(), Error> {
        let width = Self::size()?.width;
        Self::print_row(
            row,
            &format!(
                // 使用宽度填充并确保文本符合终端宽度
                "{}{:width$.width$}{}",
                // 开始反转颜色
                Attribute::Reverse,
                // 实际要显示的文本
                line_text,
                // 结束反转颜色，恢复默认样式
                Attribute::Reset
            ),
        )
    }

    /// 打印
    pub fn print(str: &str) -> Result<(), Error> {
        Self::queue_command(Print(str))?;
        Ok(())
    }

    /// 获取终端size
    /// 对于 `usize` < `u16` 的系统：
    /// * 一个表示终端大小的 `Size`。任何坐标 `z` 如果 `usize` < `z` < `u16`，则会被截断为 `usize`。
    pub fn size() -> Result<Size, Error> {
        let (width_u16, height_u16) = size()?;
        // clippy::as_conversions: See doc above
        #[allow(clippy::as_conversions)]
        let height = height_u16 as usize;
        // clippy::as_conversions: See doc above
        #[allow(clippy::as_conversions)]
        let width = width_u16 as usize;
        Ok(Size { height, width })
    }
    
    /// 执行刷新缓冲区
    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }

    /// 执行命令
    fn queue_command<T: Command>(command: T) -> Result<(), Error> {
        queue!(stdout(), command)?;
        Ok(())
    }
}