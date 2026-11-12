use crate::system::ocr_line::OcrLine;
use serde::{Deserialize, Serialize};

/// Text recognized from one captured image.
///
/// Every rectangle is in image pixels, the same coordinate space a
/// `screenshot` result's `capture_id` maps back to screen coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub lines: Vec<OcrLine>,
}
