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
    // 字素字节索引
    start_byte_idx: usize,
}

#[derive(Default)]
pub struct Line {
    fragments: Vec<TextFragment>,
    string: String,
}

impl Line {
    pub fn from(line_str: &str) -> Self {
        let fragments = Self::str_to_fragments(line_str);
        Self { 
            fragments,
            string: String::from(line_str)
        }
    }

    fn str_to_fragments(line_str: &str) -> Vec<TextFragment> {
        // 使用 `.graphemes(true)` 将字符串拆分成图形单元（grapheme clusters）
        // 图形单元是人类可感知的字符单位，可能由多个 Unicode 码点组成
        line_str
            .grapheme_indices(true)
            .map(|(byte_idx, grapheme)| {
                let (replacement, rendered_width) = Self::get_replacement_character(grapheme)
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
                    start_byte_idx: byte_idx,
                }
            })
            .collect()
    }

    /// 重新构建 fragment
    fn rebuild_fragments(&mut self) {
        self.fragments = Self::str_to_fragments(&self.string);
    }

    /// 处理替换字符
    fn get_replacement_character(for_str: &str) -> Option<char> {
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

    /// 内容长度
    pub fn grapheme_count(&self) -> usize {
        self.fragments.len()
    }

    /// 计算宽度
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

    /// 获取行宽度
    pub fn width(&self) -> usize {
        self.width_until(self.grapheme_count())
    }
    
    /// 插入字符
    pub fn insert_char(&mut self, character: char, at: usize) {
        // 尝试检索相应的片段,直接操作string
        if let Some(fragment) = self.fragments.get(at) {
            // 根据字素索引插入
            self.string.insert(fragment.start_byte_idx, character);
        } else {
            // 添加到末尾
            self.string.push(character);
        }

        // 通过rebuild方法将string重新构建成fragments
        self.rebuild_fragments();
    }

    /// 追加字符
    pub fn append_char(&mut self, character: char) {
        self.insert_char(character, self.grapheme_count());
    }
    
    /// 删除指定位置字符
    pub fn delete(&mut self, at: usize) {
        // 尝试检索相应的片段,直接操作string
        if let Some(fragment) = self.fragments.get(at) {
            // 获取字素开始索引
            let start = fragment.start_byte_idx;
            // 根据grapheme 簇长度计算结束索引
            let end = fragment
                .start_byte_idx
                .saturating_add(fragment.grapheme.len());
            // 通过索引范围移除
            self.string.drain(start..end);
            // rebuild重生构建fragments
            self.rebuild_fragments();
        }
    }

    /// 删除最后的字符
    pub fn delete_last(&mut self) {
        self.delete(self.grapheme_count().saturating_sub(1));
    }

    /// 追加内容
    pub fn append(&mut self, other: &Self) {
        self.string.push_str(&other.string);
        self.rebuild_fragments();
    }

    /// 分隔方法：在指定的图形单元索引处将行分割为两部分。
    pub fn split(&mut self, at: usize) -> Self {
        // 尝试检索相应的片段,直接操作string
        if let Some(fragment) = self.fragments.get(at) {
            // 分隔后进行rebuild,返回后剩余的
            let remainder = self.string.split_off(fragment.start_byte_idx);
            self.rebuild_fragments();
            Self::from(&remainder)
        } else {
            Self::default()
        }
    }

    /// 将给定的字节索引转换为字素索引
    fn byte_idx_to_grapheme_idx(&self, byte_idx: usize) -> usize {
        for (grapheme_idx, fragment) in self.fragments.iter().enumerate() {
            if fragment.start_byte_idx >= byte_idx {
                return grapheme_idx;
            }
        }
        #[cfg(debug_assertions)]
        {
            panic!("Invalid byte_idx passed to byte_idx_to_grapheme_idx: {byte_idx:?}");
        }
        #[cfg(not(debug_assertions))]
        {
            0
        }
    }

    /// 搜索
    pub fn search(&self, query: &str) -> Option<usize> {
        // 获取字符串中对应字符内容的字节索引
        self.string
            .find(query)
            .map(|byte_idx| self.byte_idx_to_grapheme_idx(byte_idx))
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