use crate::Rect;
use serde::{Deserialize, Serialize};

/// One recognized word and the image-pixel rectangle it occupies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrWord {
    pub text: String,
    pub rect: Rect,
}
