use std::{fmt, ops::{Deref, Range}};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

type GraphemeIdx = usize;
type ByteIdx = usize;

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

#[derive(Clone)]
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

#[derive(Default, Clone)]
pub struct Line {
    fragments: Vec<TextFragment>,
    string: String,
}

impl Line {
    pub fn from(line_str: &str) -> Self {
        debug_assert!(line_str.is_empty() || line_str.lines().count() == 1);
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
    pub fn get_visible_graphemes(&self, range: Range<GraphemeIdx>) -> String {
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
    pub fn grapheme_count(&self) -> GraphemeIdx {
        self.fragments.len()
    }

    /// 计算宽度
    pub fn width_until(&self, grapheme_index: GraphemeIdx) -> GraphemeIdx {
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
    pub fn width(&self) -> GraphemeIdx {
        self.width_until(self.grapheme_count())
    }
    
    /// 插入字符
    pub fn insert_char(&mut self, character: char, at: GraphemeIdx) {
        debug_assert!(at.saturating_sub(1) <= self.grapheme_count());
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
    pub fn delete(&mut self, at: GraphemeIdx) {
        debug_assert!(at <= self.grapheme_count());
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
    pub fn split(&mut self, at: GraphemeIdx) -> Self {
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
    fn byte_idx_to_grapheme_idx(&self, byte_idx: ByteIdx) -> GraphemeIdx {
        debug_assert!(byte_idx <= self.string.len());
        self.fragments
            .iter()
            // position 确保只返回在传递的闭包上返回 true 的第一个元素。
            .position(|fragment| fragment.start_byte_idx >= byte_idx)
            .map_or_else(
                || {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Fragment not found for byte index: {byte_idx:?}");
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        0
                    }
                },
                |grapheme_idx| grapheme_idx,
            )
    }

    /// 将给定的字素索引转换为字节索引
    fn grapheme_idx_to_byte_idx(&self, grapheme_idx: GraphemeIdx) -> ByteIdx {
        debug_assert!(grapheme_idx <= self.grapheme_count());
        if grapheme_idx == 0 || self.grapheme_count() == 0 {
            return 0;
        }
        self.fragments.get(grapheme_idx).map_or_else(
            || {
                #[cfg(debug_assertions)]
                {
                    panic!("Fragment not found for grapheme index: {grapheme_idx:?}");
                }
                #[cfg(not(debug_assertions))]
                {
                    0
                }
            },
            |fragment| fragment.start_byte_idx,
        )
    }

    /// 向下搜索给定查询字符串的位置。
    ///
    /// # 参数
    /// - `query`: 要搜索的字符串。
    /// - `from_grapheme_idx`: 搜索的起始位置（字素索引）。
    ///
    /// # 返回值
    /// 如果找到匹配项，则返回匹配项的字素索引；否则返回 `None`。
    ///
    /// # 逻辑说明
    /// 该方法从指定位置开始向下搜索，直到字符串末尾，查找第一个出现的匹配项。
    pub fn search_forward(&self, query: &str, from_grapheme_idx: GraphemeIdx,) -> Option<GraphemeIdx> {
        // 确保起始位置在有效范围内
        debug_assert!(from_grapheme_idx <= self.grapheme_count());
        // 如果起始位置正好是字符串的末尾，则直接返回 None，因为没有更多内容可搜索
        if from_grapheme_idx == self.grapheme_count() {
            return None;
        }
        // 将字素索引转换为字节索引，用于字符串切片操作
        let start_byte_idx = self.grapheme_idx_to_byte_idx(from_grapheme_idx);
        // 获取从起始位置到字符串末尾的子字符串，并进行搜索
        self.string
            .get(start_byte_idx..)
            // 进行搜索
            .and_then(|substr| substr.find(query))
            // 加上前面截断用的索引, 再将对应的字节索引转换为字素索引返回
            .map(|byte_idx| self.byte_idx_to_grapheme_idx(byte_idx.saturating_add(start_byte_idx)))
    }

    /// 向上搜索给定查询字符串的位置。
    ///
    /// # 参数
    /// - `query`: 要搜索的字符串。
    /// - `from_grapheme_idx`: 搜索的起始位置（图形符号索引）。
    ///
    /// # 返回值
    /// 如果找到匹配项，则返回匹配项的图形符号索引；否则返回 `None`。
    ///
    /// # 逻辑说明
    /// 该方法从指定位置开始向上搜索，直到字符串开头，查找最后一个出现的匹配项。
    pub fn search_backward(&self, query: &str, from_grapheme_idx: GraphemeIdx,) -> Option<GraphemeIdx> {
        // 确保在范围内
        debug_assert!(from_grapheme_idx <= self.grapheme_count());
        // 如果起始位置正好是字符串的开头，则直接返回 None，因为没有更多内容可搜索
        if from_grapheme_idx == 0 {
            return None;
        }
        // 获取结束字节索引：如果起始位置正好是字符串的末尾，则使用整个字符串长度；
        // 否则，将图形符号索引转换为字节索引
        let end_byte_index = if from_grapheme_idx == self.grapheme_count() {
            self.string.len()
        } else {
            self.grapheme_idx_to_byte_idx(from_grapheme_idx)
        };
        // 获取从字符串开头到结束字节索引的子字符串
        self.string
            .get(..end_byte_index)
            // 查找所有匹配项并取最后一个，实现反向搜索
            .and_then(|substr| substr.match_indices(query).last())
            // 将找到的字节索引转换回图形符号索引并返回
            .map(|(index, _)| self.byte_idx_to_grapheme_idx(index))
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

// 实现Deref trait,让它可以像指针一样解引用
impl Deref for Line {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.string
    }
}