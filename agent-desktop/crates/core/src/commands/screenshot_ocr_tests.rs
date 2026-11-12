use super::find_match;
use crate::{OcrLine, OcrResult, OcrWord, Rect};

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
fn matches_a_whole_line_when_no_word_spans_the_query() {
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
fn matches_a_spaced_line_directly_before_compacting() {
    let result = OcrResult {
        text: "你 好 世 界".into(),
        lines: vec![chinese_line()],
    };
    let found = find_match(&result, "好 世").expect("the spaced line text contains the query");
    assert_eq!(found["line"], "你 好 世 界");
    assert_eq!(found["width"], 40.0);
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
