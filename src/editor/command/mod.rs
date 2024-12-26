use crossterm::event::Event;
use std::convert::TryFrom;

use super::Size;
pub use edit::Edit;
pub use movecommand::Move;
pub use system::System;

mod edit;
mod movecommand;
mod system;

/// 操作命令枚举
#[derive(Clone, Copy)]
pub enum Command {
    Move(Move),
    Edit(Edit),
    System(System),
}

// clippy::as_conversions: Will run into problems for rare edge case systems where usize < u16
#[allow(clippy::as_conversions)]
impl TryFrom<Event> for Command {
    type Error = String;
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Key(key_event) => {
                // 尝试将 key_event 转换为 Edit 命令枚举。如果成功，则将其包装到 Command::Edit 中。
                Edit::try_from(key_event)
                    .map(Command::Edit)
                    // 上一个转换失败，就转换成 Move
                    .or_else(|_| Move::try_from(key_event).map(Command::Move))
                    // 上一个转换失败，就转换成 System
                    .or_else(|_| System::try_from(key_event).map(Command::System))
                    // 都不行就格式化信息返回Err
                    .map_err(|_err| format!("Event not supported: {key_event:?}"))
            },
            Event::Resize(width_u16, height_u16) => {
                Ok(Self::System(System::Resize(Size {
                    height: height_u16 as usize,
                    width: width_u16 as usize,
                })))
            }
            _ => Err(format!("Event not supported: {event:?}")),
        }
    }
}