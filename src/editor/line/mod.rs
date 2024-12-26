use crate::prelude::*;
use std::{cmp::min, fmt::{self, Display}, ops::{Deref, Range}};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use graphemewidth::GraphemeWidth;
use textfragment::TextFragment;

use super::{AnnotatedString, AnnotationType};

mod graphemewidth;
mod textfragment;

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
        // 使用 `.graphemes(true)` 将字符串拆分成字素（grapheme clusters）
        // 字素是人类可感知的字符单位，可能由多个 Unicode 码点组成
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
                    start: byte_idx,
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

    /// 根据列索引获取可展示的内容
    pub fn get_visible_graphemes(&self, range: Range<ColIdx>) -> String {
        self.get_annotated_visible_substr(range, None, None).to_string()
    }

    /// 获取给定列索引范围内的带注释字符串。
    ///
    /// 注意：列索引不同于图形符号索引：
    /// - 一个图形符号可以占用2个列宽。
    ///
    /// # 参数
    /// - `range`: 获取带注释字符串的列索引范围。
    /// - `query`: 要高亮显示在带注释字符串中的查询字符串。
    /// - `selected_match`: 要高亮显示在带注释字符串中的选定匹配项。仅在查询字符串不为空时应用。
    ///
    /// # 返回值
    /// 返回一个带注释的字符串 (`AnnotatedString`)。
    pub fn get_annotated_visible_substr(
        &self,
        range: Range<ColIdx>,
        query: Option<&str>,
        selected_match: Option<GraphemeIdx>,
    ) -> AnnotatedString {
        // 如果起始列索引大于或等于结束列索引，则返回默认的空带注释字符串
        if range.start >= range.end {
            return AnnotatedString::default();
        }

        // 创建一个新的带注释字符串
        let mut result = AnnotatedString::from(&self.string);

        // 根据搜索结果对字符串进行注释
        if let Some(query) = query {
            if !query.is_empty() {
                // 查找所有匹配项，并为每个匹配项添加注释
                self.find_all(query, 0..self.string.len()).iter().for_each(
                    |(start_byte_idx, grapheme_idx)| {
                        if let Some(selected_match) = selected_match {
                            if *grapheme_idx == selected_match {
                                // 如果是选定匹配项，则使用特殊注释类型（SelectedMatch）
                                result.add_annotation(
                                    AnnotationType::SelectedMatch,
                                    *start_byte_idx,
                                    start_byte_idx.saturating_add(query.len()),
                                );
                                return;
                            }
                        }
                        // 否则使用普通匹配注释类型（Match）
                        result.add_annotation(
                            AnnotationType::Match,
                            *start_byte_idx,
                            start_byte_idx.saturating_add(query.len()),
                        );
                    },
                );
            }
        }

        // 插入替换字符，并根据需要截断字符串。
        // 反向处理是为了确保在替换字符宽度不同的情况下，字节索引仍然正确。

        // 因为要反向处理，所以开始位置初始设置为总宽度
        let mut fragment_start = self.width(); 
        for fragment in self.fragments.iter().rev() {
            // 将片段的结尾设置为fragment_start
            let fragment_end = fragment_start;
            // 减去片段渲染长度,计算出该片段的开始位置
            fragment_start = fragment_start.saturating_sub(fragment.rendered_width.into());

            // 如果当前片段尚未进入可见范围，则跳过处理
            if fragment_start > range.end {
                continue;
            }

            // 如果片段部分可见（右边缘超出范围），则用省略号替换
            if fragment_start < range.end && fragment_end > range.end {
                result.replace(fragment.start, self.string.len(), "⋯");
                continue;
            } else if fragment_start == range.end {
                // 如果正好到达可见范围的末尾，则截断右侧
                result.truncate_right_from(fragment.start);
                continue;
            }

            // 如果片段的右边缘小于可见范围的起始位置，则移除左侧
            if fragment_end <= range.start {
                result.truncate_left_until(fragment.start.saturating_add(fragment.grapheme.len()));
                break; // 剩余片段都不可见，结束处理
            } else if fragment_start < range.start && fragment_end > range.start {
                // 如果片段与可见范围的起始位置重叠，则移除左侧并添加省略号
                result.replace(
                    0,
                    fragment.start.saturating_add(fragment.grapheme.len()),
                    "⋯",
                );
                break; // 剩余片段都不可见，结束处理
            }

            // 如果片段完全在可见范围内，则根据需要应用替换字符
            if fragment_start >= range.start && fragment_end <= range.end {
                if let Some(replacement) = fragment.replacement {
                    let start_byte_idx = fragment.start;
                    let end_byte_idx = start_byte_idx.saturating_add(fragment.grapheme.len());
                    result.replace(start_byte_idx, end_byte_idx, &replacement.to_string());
                }
            }
        }

        result
    }

    /// 内容长度
    pub fn grapheme_count(&self) -> GraphemeIdx {
        self.fragments.len()
    }

    /// 计算宽度
    pub fn width_until(&self, grapheme_index: GraphemeIdx) -> ColIdx {
        // 计算到指定字素为止的总宽度
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
    pub fn width(&self) -> ColIdx {
        self.width_until(self.grapheme_count())
    }
    
    /// 插入字符
    pub fn insert_char(&mut self, character: char, at: GraphemeIdx) {
        debug_assert!(at.saturating_sub(1) <= self.grapheme_count());
        // 尝试检索相应的片段,直接操作string
        if let Some(fragment) = self.fragments.get(at) {
            // 根据字素索引插入
            self.string.insert(fragment.start, character);
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
            let start = fragment.start;
            // 根据grapheme 簇长度计算结束索引
            let end = fragment
                .start
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

    /// 分隔方法：在指定的字素索引处将行分割为两部分。
    pub fn split(&mut self, at: GraphemeIdx) -> Self {
        // 尝试检索相应的片段,直接操作string
        if let Some(fragment) = self.fragments.get(at) {
            // 分隔后进行rebuild,返回后剩余的
            let remainder = self.string.split_off(fragment.start);
            self.rebuild_fragments();
            Self::from(&remainder)
        } else {
            Self::default()
        }
    }

    /// 将给定的字节索引转换为字素索引
    fn byte_idx_to_grapheme_idx(&self, byte_idx: ByteIdx) -> Option<GraphemeIdx> {
        if byte_idx > self.string.len() {
            return None;
        }
        self.fragments
            .iter()
            // position 确保只返回在传递的闭包上返回 true 的第一个元素。
            .position(|fragment| fragment.start >= byte_idx)
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
            |fragment| fragment.start,
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
        let start = self.grapheme_idx_to_byte_idx(from_grapheme_idx);
        // 获取从起始位置到字符串末尾的子字符串，并进行搜索，取结果中的第一个
        self.find_all(query, start..self.string.len())
            .first()
            .map(|(_, grapheme_idx)| *grapheme_idx)
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
        // 查找所有匹配项并取最后一个，实现反向搜索
        self.find_all(query, 0..end_byte_index)
            .last()
            .map(|(_, grapheme_idx)| *grapheme_idx)
    }

    /// 根据给定字节索引范围搜索所有匹配的内容，最后返回匹配项的字节索引和图形符号索引的集合。
    ///
    /// # 参数
    /// - `query`: 要搜索的查询字符串。
    /// - `range`: 搜索的字节索引范围。
    ///
    /// # 返回值
    /// 返回一个包含匹配项的字节索引和图形符号索引的向量 (`Vec<(ByteIdx, GraphemeIdx)>`)。
    fn find_all(&self, query: &str, range: Range<ByteIdx>) -> Vec<(ByteIdx, GraphemeIdx)> {
        let end = min(range.end, self.string.len());
        let start = range.start;
        debug_assert!(start <= end);
        debug_assert!(start <= self.string.len());
        // 截取得到所需的 substring。如果未找到，则返回一个空 vector
        self.string.get(start..end).map_or_else(Vec::new, |substr| {
            // 从范围截取的字符串中进行匹配比较
            let potential_matches: Vec<ByteIdx> = substr
                // 查找所有匹配项，返回迭代器 (相对起始字节索引, 匹配字符串)
                .match_indices(query)
                .map(|(relative_start_idx, _)| {
                    // 将相对字节索引转换为绝对字节索引
                    relative_start_idx.saturating_add(start)
                })
                .collect();
            // 检查潜在的匹配项并将它们映射到所需的(起始字节索引/字素索引)集合。
            self.match_grapheme_clusters(&potential_matches, query)
        })
    }

    /// 查找所有与字素边界对齐的匹配项。
    ///
    /// # 参数
    /// - `query`: 要搜索的查询字符串。
    /// - `matches`: 包含潜在匹配项的字节索引的向量，这些匹配项可能不完全与字素边界对齐。
    ///
    /// # 返回值
    /// 返回一个包含 `(byte_index, grapheme_idx)` 对的向量，每个对表示一个与字素边界对齐的匹配项，
    /// 其中 `byte_index` 是匹配项的字节索引，`grapheme_idx` 是匹配项的字素索引。
    fn match_grapheme_clusters(
        &self,
        matches: &[ByteIdx],
        query: &str,
    ) -> Vec<(ByteIdx, GraphemeIdx)> {
        // 计算查询字符串中的字素数量
        let grapheme_count = query.graphemes(true).count();

        // 遍历潜在匹配项的字节索引，并筛选出与字素边界对齐的匹配项
        matches
            .iter()
            .filter_map(|&start| {
                // 将字节索引转换为字素索引
                self.byte_idx_to_grapheme_idx(start)
                    .and_then(|grapheme_idx| {
                        // 获取从当前字素索引开始的、与查询字符串长度相等的片段
                        self.fragments
                            .get(grapheme_idx..grapheme_idx.saturating_add(grapheme_count))
                            .and_then(|fragments| {
                                // 将这些片段组合成一个字符串，并检查是否与查询字符串匹配
                                let substring = fragments
                                    .iter()
                                    .map(|fragment| fragment.grapheme.as_str())
                                    .collect::<String>();

                                // 如果组合后的字符串与查询字符串匹配，则返回匹配项的字节索引和字素索引
                                (substring == query).then_some((start, grapheme_idx))
                            })
                    })
            })
            .collect() // 收集所有符合条件的匹配项到一个向量中
    }
}

impl Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.string)
    }
}

// 实现Deref trait,让它可以像指针一样解引用
impl Deref for Line {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.string
    }
}