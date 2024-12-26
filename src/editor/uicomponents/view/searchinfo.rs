use crate::editor::{Line, Position};
use super::Location;

pub struct SearchInfo {
    // 搜索前光标所在文本位置
    pub prev_location: Location,
    // 搜索前view的滚动偏移量
    pub prev_scroll_offset: Position,
    // 搜索内容
    pub query: Option<Line>
}