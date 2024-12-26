use super::AnnotationType;

/// 注释/标注
// clippy::struct_field_names: naming the field `type` is disallowed due to type being a keyword.
#[derive(Copy, Clone, Debug)]
#[allow(clippy::struct_field_names)]
pub struct Annotation {
    // 注释/标注类型
    pub annotation_type: AnnotationType,
    // 开始字节索引
    pub start_byte_idx: usize,
    // 结束字节索引
    pub end_byte_idx: usize,
}