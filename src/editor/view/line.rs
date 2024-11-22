use std::{cmp, ops::Range};
pub struct Line {
    string: String,
}
impl Line {
    pub fn from(line_str: &str) -> Self {
        Self {
            string: String::from(line_str),
        }
    }
    pub fn get(&self, range: Range<usize>) -> String {
        let start = range.start;
        // 取较小的值,避免substring超出索引长度返回None,让它始终返回一个字符串
        let end = cmp::min(range.end, self.string.len());
        self.string.get(start..end).unwrap_or_default().to_string()
    }
}