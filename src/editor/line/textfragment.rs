use super::GraphemeWidth;

#[derive(Clone, Debug)]
pub struct TextFragment {
    // 图形单元的字符串形式
    pub grapheme: String,
    // 渲染宽度
    pub rendered_width: GraphemeWidth,
    // 替换字符（如果有）
    pub replacement: Option<char>,
    // 字素字节索引
    pub start_byte_idx: usize,
}