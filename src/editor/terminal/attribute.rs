use crossterm::style::Color;
use crate::editor::annotatedstring::AnnotationType;

/// 终端可以使用的属性
pub struct Attribute {
    // 前景字体颜色
    pub foreground: Option<Color>,
    // 背景颜色
    pub background: Option<Color>,
}

impl From<AnnotationType> for Attribute {
    fn from(annotation_type: AnnotationType) -> Self {
        match annotation_type {
            AnnotationType::Match => Self {
                foreground: Some(Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                }),
                background: Some(Color::Rgb {
                    r: 100,
                    g: 100,
                    b: 100,
                }),
            },
            AnnotationType::SelectedMatch => Self {
                foreground: Some(Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                }),
                background: Some(Color::Rgb {
                    r: 255,
                    g: 251,
                    b: 0,
                }),
            },
        }
    }
}