use super::{ScreenshotArgs, execute, find_match};
use crate::adapter::{ActionOps, InputOps, ObservationOps, ScreenshotTarget, SystemOps};
use crate::{AdapterError, Deadline, ImageBuffer, ImageFormat, OcrLine, OcrResult, OcrWord, Rect};

struct RecognizingAdapter;

impl ObservationOps for RecognizingAdapter {}
impl ActionOps for RecognizingAdapter {}
impl InputOps for RecognizingAdapter {}

impl SystemOps for RecognizingAdapter {
    fn screenshot(
        &self,
        _target: ScreenshotTarget,
        _deadline: Deadline,
    ) -> Result<ImageBuffer, AdapterError> {
        Ok(ImageBuffer {
            data: vec![1, 2, 3],
            format: ImageFormat::Png,
            width: 40,
            height: 10,
            scale_factor: 1.0,
        })
    }

    fn ocr_image(
        &self,
        _image: &ImageBuffer,
        _language: Option<&str>,
        _deadline: Deadline,
    ) -> Result<OcrResult, AdapterError> {
        Ok(OcrResult {
            text: "你 好 世 界".into(),
            lines: vec![chinese_line()],
        })
    }
}

fn screenshot_args(ocr: bool, query: Option<&str>, language: Option<&str>) -> ScreenshotArgs {
    ScreenshotArgs {
        app: None,
        window_id: None,
        screen: None,
        output_path: None,
        ocr,
        ocr_language: language.map(str::to_string),
        ocr_query: query.map(str::to_string),
    }
}

fn word(text: &str, x: f64, width: f64) -> OcrWord {
    OcrWord {
        text: text.into(),
        rect: Rect {
            x,
            y: 0.0,
            width,
            height: 10.0,
        },
    }
}

fn chinese_line() -> OcrLine {
    OcrLine {
        text: "你 好 世 界".into(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 10.0,
        },
        words: vec![
            word("你", 0.0, 10.0),
            word("好", 10.0, 10.0),
            word("世", 20.0, 10.0),
            word("界", 30.0, 10.0),
        ],
    }
}

#[test]
fn matches_a_single_word_case_insensitively() {
    let result = OcrResult {
        text: "Hello OCR 12345".into(),
        lines: vec![OcrLine {
            text: "Hello OCR 12345".into(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 130.0,
                height: 10.0,
            },
            words: vec![
                word("Hello", 0.0, 40.0),
                word("OCR", 45.0, 30.0),
                word("12345", 80.0, 50.0),
            ],
        }],
    };
    let found = find_match(&result, "ocr").expect("OCR is a word in the result");
    assert_eq!(found["text"], "OCR");
    assert_eq!(found["x"], 45.0);
    assert_eq!(found["center_x"], 60.0);
}

#[test]
fn an_exact_word_wins_over_a_larger_word_that_contains_the_query() {
    let result = OcrResult {
        text: "Bookmarks OK".into(),
        lines: vec![OcrLine {
            text: "Bookmarks OK".into(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 130.0,
                height: 10.0,
            },
            words: vec![word("Bookmarks", 0.0, 90.0), word("OK", 110.0, 20.0)],
        }],
    };
    let found = find_match(&result, "ok").expect("OK is a word in the result");
    assert_eq!(found["text"], "OK");
    assert_eq!(found["center_x"], 120.0);
}

#[test]
fn a_phrase_spanning_the_line_covers_every_word() {
    let result = OcrResult {
        text: "Hello World".into(),
        lines: vec![OcrLine {
            text: "Hello World".into(),
            rect: Rect {
                x: 5.0,
                y: 6.0,
                width: 80.0,
                height: 10.0,
            },
            words: vec![word("Hello", 5.0, 35.0), word("World", 45.0, 40.0)],
        }],
    };
    let found = find_match(&result, "hello world").expect("the line contains the query");
    assert_eq!(found["line"], "Hello World");
    assert_eq!(found["width"], 80.0);
}

#[test]
fn matches_character_words_when_the_query_has_no_spaces() {
    let result = OcrResult {
        text: "你 好 世 界".into(),
        lines: vec![chinese_line()],
    };
    let found = find_match(&result, "世界").expect("consecutive character words contain the query");
    assert_eq!(found["text"], "世界");
    assert_eq!(found["x"], 20.0);
    assert_eq!(found["width"], 20.0);
    assert_eq!(found["center_x"], 30.0);
    assert_eq!(found["center_y"], 5.0);
}

#[test]
fn a_spaced_phrase_is_centered_on_the_words_it_covers_not_on_its_line() {
    let result = OcrResult {
        text: "你 好 世 界".into(),
        lines: vec![chinese_line()],
    };
    let found = find_match(&result, "好 世").expect("the spaced line text contains the query");
    assert_eq!(found["line"], "你 好 世 界");
    assert_eq!(found["x"], 10.0);
    assert_eq!(found["width"], 20.0);
    assert_eq!(found["center_x"], 20.0);
}

#[test]
fn a_phrase_match_reports_the_query_as_it_was_typed() {
    let result = OcrResult {
        text: "Open Save As Close".into(),
        lines: vec![OcrLine {
            text: "Open Save As Close".into(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 10.0,
            },
            words: vec![
                word("Open", 0.0, 40.0),
                word("Save", 50.0, 40.0),
                word("As", 95.0, 20.0),
                word("Close", 130.0, 70.0),
            ],
        }],
    };
    let found = find_match(&result, " Save As ").expect("the phrase is in the line");
    assert_eq!(found["text"], "Save As");
    assert_eq!(found["x"], 50.0);
    assert_eq!(found["width"], 65.0);
    assert_eq!(found["center_x"], 82.5);
}

#[test]
fn a_line_without_words_still_matches_as_a_whole() {
    let result = OcrResult {
        text: "Hello World".into(),
        lines: vec![OcrLine {
            text: "Hello World".into(),
            rect: Rect {
                x: 5.0,
                y: 6.0,
                width: 80.0,
                height: 10.0,
            },
            words: Vec::new(),
        }],
    };
    let found = find_match(&result, "hello world").expect("the line text contains the query");
    assert_eq!(found["text"], "Hello World");
    assert_eq!(found["width"], 80.0);
}

#[test]
fn returns_none_for_a_missing_query_or_blank_input() {
    let result = OcrResult {
        text: "你 好 世 界".into(),
        lines: vec![chinese_line()],
    };
    assert!(find_match(&result, "missing").is_none());
    assert!(find_match(&result, "   ").is_none());
}

#[test]
fn an_ocr_flag_without_ocr_is_refused_before_the_capture() {
    for arguments in [
        screenshot_args(false, Some("世界"), None),
        screenshot_args(false, None, Some("zh-Hans")),
    ] {
        let error = execute(arguments, &RecognizingAdapter).unwrap_err();
        assert_eq!(error.code(), "INVALID_ARGS");
        assert!(error.to_string().contains("--ocr"));
    }
}

#[test]
fn a_query_that_matches_adds_the_match_beside_the_full_result() {
    let data = execute(
        screenshot_args(true, Some("世界"), None),
        &RecognizingAdapter,
    )
    .unwrap();

    assert_eq!(data["ocr_match"]["center_x"], 30.0);
    assert_eq!(data["ocr"]["text"], "你 好 世 界");
    assert_eq!(data["ocr"]["lines"][0]["words"][2]["text"], "世");
}

#[test]
fn a_query_that_matches_nothing_omits_the_match_field() {
    let data = execute(
        screenshot_args(true, Some("missing"), None),
        &RecognizingAdapter,
    )
    .unwrap();

    assert!(data.get("ocr_match").is_none());
    assert_eq!(data["ocr"]["text"], "你 好 世 界");
}
