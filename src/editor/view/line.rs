use std::ops::Range;

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

pub struct Line {
    fragments: Vec<TextFragment>
}
impl Line {
    pub fn from(line_str: &str) -> Self {
        // 使用 `.graphemes(true)` 将字符串拆分成图形单元（grapheme clusters）
        // 图形单元是人类可感知的字符单位，可能由多个 Unicode 码点组成
        let fragments = line_str
            .graphemes(true)
            .map(|grapheme| {
                // 获取当前图形单元的宽度
                let unicode_width = grapheme.width();
                // 根据宽度确定渲染宽度
                let rendered_width = match unicode_width {
                    // 宽度为 0 或 1 的图形单元被视为半宽字符
                    0 | 1 => GraphemeWidth::Half,
                    // 其他宽度的图形单元被视为全宽字符
                    _ => GraphemeWidth::Full
                };
                // 确定替换的图形单元
                let replacement = match unicode_width {
                    0 => Some('.'),
                    _ => None
                };

                TextFragment {
                    grapheme: grapheme.to_string(),
                    rendered_width,
                    replacement,
                }
            })
            .collect();
        Self { fragments }
    }

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
}