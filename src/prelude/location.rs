use super::{GraphemeIdx, LineIdx};

#[derive(Copy, Clone, Default)]
pub struct Location {
    pub grapheme_index: GraphemeIdx,
    pub line_index: LineIdx,
}