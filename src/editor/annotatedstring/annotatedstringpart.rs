use super::AnnotationType;

/// 注释/标识字符串内容(指针指向引用)
/*
 * 如果使用String字符串，我们将为每个微小的带注释的部分创建一个完整的副本，这是不必要的。
 * 我们只需返回一个指向原始字符串的指针，不需要创建副本。Rust 的编译器会确保它不被修改。
 */
#[derive(Debug)]
pub struct AnnotatedStringPart<'a> {
    pub string: &'a str,
    pub annotation_type: Option<AnnotationType>,
}