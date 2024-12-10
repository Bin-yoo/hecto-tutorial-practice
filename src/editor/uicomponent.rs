use std::io::Error;
use super::terminal::Size;

/// 定义ui组件行为方法的trait
pub trait UIComponent {
    // 标记此 UI 组件是否需要重绘
    fn mark_redraw(&mut self, value: bool);

    // 判断组件是否需要重绘
    fn needs_redraw(&self) -> bool;

    // 更新组件大小并标记为需要重绘，默认实现调用了 set_size 方法
    fn resize(&mut self, size: Size) {
        self.set_size(size);
        self.mark_redraw(true);
    }

    // 设置组件的大小，必须由每个具体组件实现
    fn set_size(&mut self, size: Size);

    // 如果组件可见且需要重绘，则绘制该组件
    fn render(&mut self, origin_y: usize) {
        if self.needs_redraw() {
            match self.draw(origin_y) {
                Ok(()) => self.mark_redraw(false),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not render component: {err:?}");
                    }
                }
            }
        }
    }
    
    // 实际绘制组件的方法，必须由每个具体组件实现
    fn draw(&mut self, origin_y: usize) -> Result<(), Error>;
}