use std::ffi::{CString, c_char, c_void};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(path: *const u16) -> *mut c_void;
    fn GetProcAddress(library: *mut c_void, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(library: *mut c_void) -> i32;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("expected cdylib path")?;
    let path = std::fs::canonicalize(path)?;
    use std::os::windows::ffi::OsStrExt;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let library = unsafe { LoadLibraryW(path.as_ptr()) };
    if library.is_null() {
        return Err("could not load cdylib".into());
    }
    let name = CString::new("ad_test_panic_boundary")?;
    let boundary = unsafe { GetProcAddress(library, name.as_ptr()) };
    let result = if boundary.is_null() {
        Err("panic boundary export missing".into())
    } else {
        let boundary: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(boundary) };
        let result = unsafe { boundary() };
        if result == -12 {
            Ok(())
        } else {
            Err(format!("unexpected result: {result}").into())
        }
    };
    unsafe {
        FreeLibrary(library);
    }
    result
}
