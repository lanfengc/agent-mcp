#[cfg(windows)]
use super::ensure_local_path;
use super::png_dimensions;

fn png_header(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0_u8; 24];
    bytes[..8].copy_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

#[test]
fn reads_png_dimensions() {
    let bytes = png_header(1080, 2400);
    assert_eq!(png_dimensions(&bytes), Some((1080, 2400)));
}

#[test]
fn rejects_non_png_data() {
    assert_eq!(png_dimensions(&[]), None);
    assert_eq!(png_dimensions(b"not a png at all, really"), None);
    let mut bytes = png_header(1, 2);
    bytes[1] = 0;
    assert_eq!(png_dimensions(&bytes), None);
}

#[cfg(windows)]
#[test]
fn rejects_network_screenshot_paths() {
    let error = ensure_local_path(std::path::Path::new(r"\\server\share\shot.png"))
        .expect_err("network path");
    assert_eq!(error.code, "PERM_DENIED");
    assert!(ensure_local_path(std::path::Path::new(r"C:\temp\shot.png")).is_ok());
}
