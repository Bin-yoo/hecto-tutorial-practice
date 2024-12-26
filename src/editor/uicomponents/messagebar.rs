use std::{io::Error, time::{Duration, Instant}};
use super::UIComponent;
use super::super::{
    Terminal,
    Size
};

const DEFAULT_DURATION: Duration = Duration::new(5, 0);

struct Message {
    text: String,
    time: Instant,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            text: String::new(),
            time: Instant::now(),
        }
    }
}

impl Message {
    fn is_expired(&self) -> bool {
        Instant::now().duration_since(self.time) > DEFAULT_DURATION
    }
}

#[derive(Default)]
pub struct MessageBar {
    // 当前消息
    current_message: Message,
    needs_redraw: bool,
    // 用来确保隐藏消息
    cleared_after_expiry: bool,
}

impl MessageBar {
    /// 更新消息栏
    pub fn update_message(&mut self, new_message: &str) {
        self.current_message = Message {
            text: new_message.to_string(),
            time: Instant::now(),
        };
        self.cleared_after_expiry = false;
        self.set_needs_redraw(true);
    }
}

impl UIComponent for MessageBar {
    fn set_needs_redraw(&mut self, value: bool) {
        self.needs_redraw = value;
    }

    fn needs_redraw(&self) -> bool {
        // 如果当前消息已过期，但我们尚未清除它，则返回 true（因为我们需要重绘）
        // 否则返回 self.needs_redraw
        (!self.cleared_after_expiry && self.current_message.is_expired()) || self.needs_redraw
    }

    fn set_size(&mut self, _: Size) {}

    fn draw(&mut self, origin: usize) -> Result<(), Error> {
        // 如果过期了，需要写入一次空字符串 "" 来清除消息。
        // 为了避免清除不必要的内容，需要记录下已经清除过期消息。
        if self.current_message.is_expired() {
            self.cleared_after_expiry = true; 
        }

        // 如果过期了就渲染空字符串,否则取消息内容渲染
        let message = if self.current_message.is_expired() {
            ""
        } else {
            &self.current_message.text
        };
        Terminal::print_row(origin, message)
    }
}