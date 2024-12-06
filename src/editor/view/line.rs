use std::{fmt, ops::Range};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Copy, Clone)]
enum GraphemeWidth {
    Half,
    Full,
}
impl GraphemeWidth {
    const fn saturating_add(self, other: usize) -> usize {
        match self {
            Self::Half => other.saturating_add(1),
            Self::Full => other.saturating_add(2),
        }
    }
}
struct TextFragment {
    // 图形单元的字符串形式
    grapheme: String,
    // 渲染宽度
    rendered_width: GraphemeWidth,
    // 替换字符（如果有）
    replacement: Option<char>,
}

#[derive(Default)]
pub struct Line {
    fragments: Vec<TextFragment>
}

impl Line {
    pub fn from(line_str: &str) -> Self {
        let fragments = Self::str_to_fragments(line_str);
        Self { fragments }
    }

    fn str_to_fragments(line_str: &str) -> Vec<TextFragment> {
        // 使用 `.graphemes(true)` 将字符串拆分成图形单元（grapheme clusters）
        // 图形单元是人类可感知的字符单位，可能由多个 Unicode 码点组成
        line_str
            .graphemes(true)
            .map(|grapheme| {
                let (replacement, rendered_width) = Self::replacement_character(grapheme)
                    .map_or_else(
                        // 如果转换的函数返回None就进行处理
                        || {
                            let unicode_width = grapheme.width();
                            let rendered_width = match unicode_width {
                                0 | 1 => GraphemeWidth::Half,
                                _ => GraphemeWidth::Full,
                            };
                            (None, rendered_width)
                        }, 
                        // Some(x)有值就直接用
                        |replacement| (Some(replacement), GraphemeWidth::Half),
                    );

                TextFragment {
                    grapheme: grapheme.to_string(),
                    rendered_width,
                    replacement,
                }
            })
            .collect()
    }

    // 处理替换字符
    fn replacement_character(for_str: &str) -> Option<char> {
        let width = for_str.width();
        match for_str {
            // 空格不用替换
            " " => None,
            // tab制表符换成空格
            "\t" => Some(' '),
            // 可见空白字符（如全角空格）替换为特殊字符 '␣'
            _ if width > 0 && for_str.trim().is_empty() => Some('␣'),
            // 不可见字符（如零宽字符）替换为特殊字符 '▯'
            _ if width == 0 => {
                let mut chars = for_str.chars();
                if let Some(ch) = chars.next() {
                    // 检查第一个字符是否是控制字符(\r, \n, \t 等)，且是单个字符
                    if ch.is_control() && chars.next().is_none() {
                        return Some('▯');
                    }
                }
                Some('.')
            }
            _ => None
        }
    }

    // 获取可展示的内容
    pub fn get_visible_graphemes(&self, range: Range<usize>) -> String {
        if range.start >= range.end {
            return String::new();
        }

        let mut result = String::new();
        let mut current_pos = 0;
        for fragment in &self.fragments {
            // 计算图形单元的结束位置
            let fragment_end = fragment.rendered_width.saturating_add(current_pos);
            // 如果当前位置超过范围结束位置，停止遍历
            if current_pos >= range.end {
                break;
            }
            // 确定当前图形单元是否部分或全部在指定范围内
            if fragment_end > range.start {
                if fragment_end > range.end || current_pos < range.start {
                    // 超出的截断展示...
                    result.push('⋯');
                } else if let Some(char) = fragment.replacement {
                    // 有替换字符的展示替换字符
                    result.push(char);
                } else {
                    // 否则使用图形单元本身
                    result.push_str(&fragment.grapheme);
                }
            }
            current_pos = fragment_end
        }
        result
    }

    pub fn grapheme_count(&self) -> usize {
        self.fragments.len()
    }

    pub fn width_until(&self, grapheme_index: usize) -> usize {
        // 计算到指定图形单元为止的总宽度
        self.fragments
            .iter()
            .take(grapheme_index)
            .map(|fragment| {
                match fragment.rendered_width {
                    GraphemeWidth::Half => 1,
                    GraphemeWidth::Full => 2
                }
            })
            .sum()
    }
    
    // 插入字符
    pub fn insert_char(&mut self, character: char, at: usize) {
        let mut result = String::new();

        // 遍历当前行内容
        for (index, fragment) in self.fragments.iter_mut().enumerate() {
            // 在对应插入位置push到result字符串中
            if index == at {
                result.push(character);
            }
            // 将原本的东西丢进去
            result.push_str(&fragment.grapheme);
        }

        // 等于或超出末尾就直接push
        if at >= self.fragments.len() {
            result.push(character);
        }

        // 经过后保存
        self.fragments = Self::str_to_fragments(&result);
    }
    
    pub fn delete(&mut self, at: usize) {
        let mut result = String::new();

        // 遍历当前行内容
        for (index, fragment) in self.fragments.iter_mut().enumerate() {
            // 非对应位置的全放进去,及通过忽略对应位置内容来达到删除的效果
            if index != at {
                result.push_str(&fragment.grapheme);
            }
        }

        // 经过后保存
        self.fragments = Self::str_to_fragments(&result);
    }

    // 追加内容
    pub fn append(&mut self, other: &Self) {
        let mut concat = self.to_string();
        concat.push_str(&other.to_string());
        self.fragments = Self::str_to_fragments(&concat);
    }

    // 分隔方法：在指定的图形单元索引处将行分割为两部分。
    pub fn split(&mut self, at: usize) -> Self {
        // 如果提供的索引超出当前行中图形单元的数量，则返回一个空的新行。
        if at > self.fragments.len() {
            return Self::default();
        }

        // 使用 Vec 的 split_off 方法来获取从 'at' 索引开始的所有片段，
        // 这个方法会修改原始的 'self.fragments'，使其只包含前 'at' 个片段，
        // 并返回一个新的 Vec，包含剩余的片段。
        let remainder = self.fragments.split_off(at);

        // 创建一个新的 Line 实例，并将其 fragments 设置为从原始行中分离出来的片段。
        // 原始行现在包含了分割点之前的内容，新行包含了分割点之后的内容。
        Self {
            fragments: remainder
        }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let result: String = self
            .fragments
            .iter()
            .map(|fragment| fragment.grapheme.clone())
            .collect();
        write!(formatter, "{result}")
    }
}