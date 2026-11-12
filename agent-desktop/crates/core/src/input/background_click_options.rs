/// Options for a background click delivered as window messages rather than
/// synthesized pointer input.
#[derive(Debug, Clone)]
pub struct BackgroundClickOptions {
    pub button: crate::MouseButton,
    pub count: u32,
    pub restore_minimized: bool,
    pub cursor_fallback: bool,
    pub semantic: bool,
}
