use std::{
    alloc::{GlobalAlloc, Layout, System},
    ops::Deref,
    os::raw::c_char,
};

#[derive(Debug)]
pub struct Error {
    inner: anyhow::Error,
}

impl Error {
    pub(crate) fn new_ptr(e: anyhow::Error) -> *mut Error {
        Box::into_raw(Box::new(Error { inner: e }))
    }
}

impl Deref for Error {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target { &self.inner }
}

#[no_mangle]
pub unsafe extern "C" fn rune_error_to_string(
    error: *mut Error,
) -> *const c_char {
    let error = &*error;
    let message = error.to_string();
    c_string(&message)
}

#[no_mangle]
pub unsafe extern "C" fn rune_error_backtrace(
    error: *mut Error,
) -> *const c_char {
    let error = &*error;
    let message = format!("{:?}", error);
    c_string(&message)
}

#[no_mangle]
pub unsafe extern "C" fn rune_error_free(error: *mut Error) {
    if error.is_null() {
        return;
    }

    let _ = Box::from_raw(error);
}

fn c_string(s: &str) -> *const c_char {
    let len = s.len() + 1;
    let layout = Layout::array::<u8>(len).unwrap();

    unsafe {
        let dest = System.alloc(layout);

        std::ptr::copy_nonoverlapping(s.as_ptr(), dest, s.len());
        // don't forget the null pointer
        dest.add(len).add(1).write(0);

        dest.cast()
    }
}
