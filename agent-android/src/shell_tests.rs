use crate::shell::quote;

#[test]
fn quotes_arguments_for_the_device_shell() {
    assert_eq!(quote("hello world"), "'hello world'");
    assert_eq!(quote("a&b;c"), "'a&b;c'");
    assert_eq!(quote("$HOME"), "'$HOME'");
    assert_eq!(quote(r"a\b"), r"'a\b'");
    assert_eq!(quote("it's"), "'it'\\''s'");
}

#[test]
fn preserves_empty_and_unicode_arguments() {
    assert_eq!(quote(""), "''");
    assert_eq!(quote("你好"), "'你好'");
}
