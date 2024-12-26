use std::cmp::min;
use super::{AnnotatedString, AnnotatedStringPart};

/// 注释/标注字符串迭代器
///
/// # 字段
/// - `annotated_string`: 引用一个带注释的字符串。
/// - `current_idx`: 当前迭代的字节索引。
pub struct AnnotatedStringIterator<'a> {
    // 使用'a生命周期，声明对 AnnotatedString 的引用的生命周期至少应该与 Iterator 本身一样长。
    pub annotated_string: &'a AnnotatedString,
    pub current_idx: usize,
}

// 实现带生命周期的迭代器trait
impl<'a> Iterator for AnnotatedStringIterator<'a> {
    type Item = AnnotatedStringPart<'a>;

    /// 返回迭代器的下一个元素
    fn next(&mut self) -> Option<Self::Item> {
        // 如果当前索引已经超出字符串长度，则返回 None，表示迭代结束
        if self.current_idx >= self.annotated_string.string.len() {
            return None;
        }
        // 查找当前有效的注释（即包含当前索引的注释）
        if let Some(annotation) = self
            .annotated_string
            .annotations
            .iter()
            .filter(|annotation| {
                annotation.start_byte_idx <= self.current_idx
                    && annotation.end_byte_idx > self.current_idx
            })
            .last()
        {
            // 确定注释的结束位置，并确保不超过字符串长度
            let end_idx = min(annotation.end_byte_idx, self.annotated_string.string.len());
            let start_idx = self.current_idx;

            // 更新当前索引到注释的结束位置
            self.current_idx = end_idx;

            // 返回包含注释类型的字符串片段
            return Some(AnnotatedStringPart {
                string: &self.annotated_string.string[start_idx..end_idx],
                annotation_type: Some(annotation.annotation_type),
            });
        }
        // 如果没有找到有效注释，则查找最近的注释边界
        let mut end_idx = self.annotated_string.string.len();
        for annotation in &self.annotated_string.annotations {
            if annotation.start_byte_idx > self.current_idx && annotation.start_byte_idx < end_idx {
                end_idx = annotation.start_byte_idx;
            }
        }

        // 确定无注释部分的结束位置
        let start_idx = self.current_idx;
        self.current_idx = end_idx;

        // 返回不包含注释类型的字符串片段
        Some(AnnotatedStringPart {
            string: &self.annotated_string.string[start_idx..end_idx],
            annotation_type: None,
        })
    }
}