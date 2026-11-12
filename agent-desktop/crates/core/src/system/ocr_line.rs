use crate::{Rect, system::ocr_word::OcrWord};
use serde::{Deserialize, Serialize};

/// One recognized line, its words, and the union of their rectangles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrLine {
    pub text: String,
    pub rect: Rect,
    pub words: Vec<OcrWord>,
}
