//! Windows OCR over an already captured PNG image.
//!
//! The recognizer is the built-in `Windows.Media.Ocr` engine. It runs against
//! a `SoftwareBitmap` built from the screenshot's BGRA pixels, so callers get
//! text and image-pixel rectangles that map back through the screenshot's
//! `capture_id` without re-capturing or re-scaling.

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, ImageBuffer, OcrLine, OcrResult, OcrWord, Rect,
};
use std::time::Duration;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::Buffer;
use windows::Win32::System::WinRT::IBufferByteAccess;
use windows::core::{HSTRING, Interface};
use windows_future::{AsyncStatus, IAsyncOperation};

use crate::system::capture::png_codec::decode_png_to_bgra;

const OCR_POLL: Duration = Duration::from_millis(10);

pub(crate) fn recognize(
    image: &ImageBuffer,
    language: Option<&str>,
    deadline: Deadline,
) -> Result<OcrResult, AdapterError> {
    deadline.ensure_remaining()?;
    let (bgra, width, height) = decode_png_to_bgra(&image.data, deadline)?;
    deadline.ensure_remaining()?;
    let engine = create_engine(language)?;
    let bitmap = software_bitmap(&bgra, width, height)?;
    deadline.ensure_remaining()?;
    let operation = engine
        .RecognizeAsync(&bitmap)
        .map_err(|error| ocr_failure("OCR recognition could not start", error))?;
    let recognized = wait(operation, deadline)?;
    let text = recognized
        .Text()
        .map_err(|error| ocr_failure("OCR result text could not be read", error))?
        .to_string_lossy();
    let lines = recognized
        .Lines()
        .map_err(|error| ocr_failure("OCR result lines could not be read", error))?;
    let mut output = Vec::new();
    for line in &lines {
        deadline.ensure_remaining()?;
        let line_text = line
            .Text()
            .map_err(|error| ocr_failure("OCR line text could not be read", error))?
            .to_string_lossy();
        let words = line
            .Words()
            .map_err(|error| ocr_failure("OCR line words could not be read", error))?;
        let mut line_words = Vec::new();
        for word in &words {
            let word_text = word
                .Text()
                .map_err(|error| ocr_failure("OCR word text could not be read", error))?
                .to_string_lossy();
            let rect = word
                .BoundingRect()
                .map_err(|error| ocr_failure("OCR word rectangle could not be read", error))?;
            line_words.push(OcrWord {
                text: word_text,
                rect: Rect {
                    x: f64::from(rect.X),
                    y: f64::from(rect.Y),
                    width: f64::from(rect.Width),
                    height: f64::from(rect.Height),
                },
            });
        }
        output.push(OcrLine {
            text: line_text,
            rect: union_rect(&line_words),
            words: line_words,
        });
    }
    Ok(OcrResult {
        text,
        lines: output,
    })
}

fn create_engine(language: Option<&str>) -> Result<OcrEngine, AdapterError> {
    let result = match language {
        Some(tag) => {
            let language = Language::CreateLanguage(&HSTRING::from(tag))
                .map_err(|error| language_error(tag, error))?;
            OcrEngine::TryCreateFromLanguage(&language)
        }
        None => OcrEngine::TryCreateFromUserProfileLanguages(),
    };
    result.map_err(|error| {
        AdapterError::new(
            ErrorCode::ActionNotSupported,
            "No Windows OCR engine is available for the requested language",
        )
        .with_suggestion(
            "Install the Windows language pack with OCR support, or pass --ocr-language with an installed tag such as zh-Hans or en-US",
        )
        .with_platform_detail(error.to_string())
    })
}

fn language_error(tag: &str, error: windows::core::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        format!("OCR language '{tag}' is not installed or is not a valid language tag"),
    )
    .with_suggestion(
        "Use a BCP-47 tag such as zh-Hans or en-US that is installed on this Windows installation",
    )
    .with_platform_detail(error.to_string())
}

fn software_bitmap(bgra: &[u8], width: u32, height: u32) -> Result<SoftwareBitmap, AdapterError> {
    let length = u32::try_from(bgra.len())
        .map_err(|_| AdapterError::internal("OCR input is larger than a WinRT buffer"))?;
    let buffer = Buffer::Create(length)
        .map_err(|error| ocr_failure("OCR buffer allocation failed", error))?;
    let access: IBufferByteAccess = buffer
        .cast()
        .map_err(|error| ocr_failure("OCR buffer access is unavailable", error))?;
    let pointer = unsafe { access.Buffer() }
        .map_err(|error| ocr_failure("OCR buffer could not be locked", error))?;
    if pointer.is_null() {
        return Err(AdapterError::internal("OCR buffer returned no memory"));
    }
    unsafe { std::ptr::copy_nonoverlapping(bgra.as_ptr(), pointer, bgra.len()) };
    buffer
        .SetLength(length)
        .map_err(|error| ocr_failure("OCR buffer length could not be set", error))?;
    let width = i32::try_from(width)
        .map_err(|_| AdapterError::internal("OCR image width exceeds the WinRT range"))?;
    let height = i32::try_from(height)
        .map_err(|_| AdapterError::internal("OCR image height exceeds the WinRT range"))?;
    SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width, height)
        .map_err(|error| ocr_failure("OCR bitmap creation failed", error))
}

fn wait<T: windows::core::RuntimeType + 'static>(
    operation: IAsyncOperation<T>,
    deadline: Deadline,
) -> Result<T, AdapterError> {
    loop {
        deadline.ensure_remaining()?;
        let status = operation
            .Status()
            .map_err(|error| ocr_failure("OCR operation status could not be read", error))?;
        match status {
            AsyncStatus::Completed => {
                return operation
                    .GetResults()
                    .map_err(|error| ocr_failure("OCR operation returned no result", error));
            }
            AsyncStatus::Error => {
                let code = operation.ErrorCode().map(|code| code.0).unwrap_or_default();
                return Err(AdapterError::new(
                    ErrorCode::ActionFailed,
                    "Windows OCR failed while recognizing the image",
                )
                .with_platform_detail(format!("HRESULT 0x{code:08X}")));
            }
            AsyncStatus::Canceled => {
                return Err(AdapterError::new(
                    ErrorCode::ActionFailed,
                    "Windows OCR was canceled",
                ));
            }
            _ => std::thread::sleep(OCR_POLL),
        }
    }
}

fn union_rect(words: &[OcrWord]) -> Rect {
    if words.is_empty() {
        return Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
    }
    let left = words
        .iter()
        .map(|word| word.rect.x)
        .fold(f64::INFINITY, f64::min);
    let top = words
        .iter()
        .map(|word| word.rect.y)
        .fold(f64::INFINITY, f64::min);
    let right = words
        .iter()
        .map(|word| word.rect.x + word.rect.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = words
        .iter()
        .map(|word| word.rect.y + word.rect.height)
        .fold(f64::NEG_INFINITY, f64::max);
    Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn ocr_failure(message: &str, error: windows::core::Error) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, message.to_string())
        .with_platform_detail(error.to_string())
}
