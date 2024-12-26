/// 注释/标注类型
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum AnnotationType {
    // 匹配：常规搜索结果。
    Match,
    // 当前选定的匹配：如果用户按 Enter，将跳转到对应地方
    SelectedMatch,
}