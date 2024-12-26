use std::{
    cmp::{max, min},
    fmt::{self, Display},
};

pub use annotationtype::AnnotationType;
use annotation::Annotation;
use annotatedstringpart::AnnotatedStringPart;
use annotatedstringiterator::AnnotatedStringIterator;

pub mod annotationtype;
mod annotation;
mod annotatedstringpart;
mod annotatedstringiterator;

#[derive(Default, Debug)]
pub struct AnnotatedString {
    // 被注释内容
    string: String,
    // 注释（在我们的项目例子中：颜色）
    annotations: Vec<Annotation>,
}

impl AnnotatedString {
    pub fn from(string: &str) -> Self {
        Self {
            string: String::from(string),
            annotations: Vec::new(),
        }
    }

    /// 新增注释
    pub fn add_annotation(
        &mut self,
        annotation_type: AnnotationType,
        start_byte_idx: usize,
        end_byte_idx: usize,
    ) {
        debug_assert!(start_byte_idx <= end_byte_idx);
        self.annotations.push(Annotation {
            annotation_type,
            start_byte_idx,
            end_byte_idx,
        });
    }

    /// 替换注释
    ///
    /// # 参数
    /// - `start_byte_idx`: 替换起始的字节索引。
    /// - `end_byte_idx`: 替换结束的字节索引。
    /// - `new_string`: 新的替换字符串。
    ///
    /// # 功能
    /// 该方法会用新的字符串替换指定范围内的内容，并相应地调整所有注释的索引。
    pub fn replace(&mut self, start_byte_idx: usize, end_byte_idx: usize, new_string: &str) {
        // 断言：确保起始索引不超过结束索引
        debug_assert!(start_byte_idx <= end_byte_idx);

        // 确保结束索引不会超出字符串长度
        let end_byte_idx = min(end_byte_idx, self.string.len());

        // 如果起始索引大于结束索引，则直接返回（无效范围）
        if start_byte_idx > end_byte_idx {
            return;
        }

        // 执行实际的字符串替换操作
        self.string.replace_range(start_byte_idx..end_byte_idx, new_string);

        // 计算被替换范围的长度
        let replaced_range_len = end_byte_idx.saturating_sub(start_byte_idx);

        // 计算新字符串与原范围长度的差异
        let len_difference = new_string.len().abs_diff(replaced_range_len);

        // 如果替换后长度没有变化，则不需要调整注释
        if len_difference == 0 {
            return;
        }

        // 检查新字符串是否比原范围短
        let shortened = new_string.len() < replaced_range_len;

        // 遍历并调整每个注释的索引
        self.annotations.iter_mut().for_each(|annotation| {
            // 调整注释的起始索引
            annotation.start_byte_idx = if annotation.start_byte_idx >= end_byte_idx {
                // 对于在替换范围之后开始的注释，根据新旧长度差异调整索引
                if shortened {
                    annotation.start_byte_idx.saturating_sub(len_difference)
                } else {
                    annotation.start_byte_idx.saturating_add(len_difference)
                }
            } else if annotation.start_byte_idx >= start_byte_idx {
                // 对于在替换范围内开始的注释，根据新旧长度差异调整索引，并限制在替换范围边界内
                if shortened {
                    max(
                        start_byte_idx,
                        annotation.start_byte_idx.saturating_sub(len_difference),
                    )
                } else {
                    min(
                        end_byte_idx,
                        annotation.start_byte_idx.saturating_add(len_difference),
                    )
                }
            } else {
                // 不需要调整
                annotation.start_byte_idx
            };

            // 调整注释的结束索引
            annotation.end_byte_idx = if annotation.end_byte_idx >= end_byte_idx {
                // 对于在替换范围之后结束的注释，根据新旧长度差异调整索引
                if shortened {
                    annotation.end_byte_idx.saturating_sub(len_difference)
                } else {
                    annotation.end_byte_idx.saturating_add(len_difference)
                }
            } else if annotation.end_byte_idx >= start_byte_idx {
                // 对于在替换范围内结束的注释，根据新旧长度差异调整索引，并限制在替换范围边界内
                if shortened {
                    max(
                        start_byte_idx,
                        annotation.end_byte_idx.saturating_sub(len_difference),
                    )
                } else {
                    min(
                        end_byte_idx,
                        annotation.end_byte_idx.saturating_add(len_difference),
                    )
                }
            } else {
                // 不需要调整
                annotation.end_byte_idx
            }
        });

        // 过滤掉无效的注释（即空注释或超出字符串长度的注释）
        self.annotations.retain(|annotation| {
            annotation.start_byte_idx < annotation.end_byte_idx
                && annotation.start_byte_idx < self.string.len()
        });
    }
}

impl Display for AnnotatedString {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.string)
    }
}

// 实现自定义迭代器
impl<'a> IntoIterator for &'a AnnotatedString {
    type Item = AnnotatedStringPart<'a>;
    type IntoIter = AnnotatedStringIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        AnnotatedStringIterator {
            annotated_string: self,
            current_idx: 0,
        }
    }
}