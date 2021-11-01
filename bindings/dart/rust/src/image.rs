use std::{
    ops::{Deref, DerefMut},
    os::raw::{c_char, c_void},
    ptr, slice,
};
use anyhow::Context;
use hotg_rune_core::Shape;
use hotg_rune_runtime::wasm3::Registrar;
use hotg_runicos_base_runtime::BaseImage;

use crate::Error;

#[derive(Default)]
pub struct Image {
    inner: BaseImage,
}
impl Image {
    pub(crate) fn into_inner(self) -> BaseImage { self.inner }
}

impl Deref for Image {
    type Target = BaseImage;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for Image {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl hotg_rune_runtime::Image<Registrar<'_>> for Image {
    fn initialize_imports(self, registrar: &mut Registrar<'_>) {
        let Image { inner } = self;
        <_ as hotg_rune_runtime::Image<Registrar<'_>>>::initialize_imports(
            inner, registrar,
        );
    }
}

#[no_mangle]
pub extern "C" fn rune_image_new() -> *mut Image {
    Box::into_raw(Box::new(Image::default()))
}

#[no_mangle]
pub extern "C" fn rune_image_new_with_defaults() -> *mut Image {
    Box::into_raw(Box::new(Image {
        inner: BaseImage::with_defaults(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rune_image_register_model_handler(
    image: *mut Image,
    mimetype: *const c_char,
    mimetype_len: usize,
    user_data: *mut c_void,
    create_model: unsafe extern "C" fn(*mut c_void),
    free: Option<unsafe extern "C" fn(*mut c_void)>,
) -> *mut Error {
    ensure!(!image.is_null());
    ensure!(!mimetype.is_null());

    let image = &mut *image;
    let mimetype = slice::from_raw_parts(mimetype as *const u8, mimetype_len);
    let mimetype = String::from_utf8_lossy(mimetype);

    let handler = ModelHandler {
        user_data,
        create_model,
        free,
    };
    image.register_model(&mimetype, handler);

    ptr::null_mut()
}

#[repr(C)]
pub struct ModelHandler {
    user_data: *mut c_void,
    create_model: unsafe extern "C" fn(*mut c_void),
    free: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl Drop for ModelHandler {
    fn drop(&mut self) {
        if let Some(free) = self.free {
            unsafe {
                free(self.user_data);
            }
        }
    }
}

impl hotg_runicos_base_runtime::ModelFactory for ModelHandler {
    fn new_model(
        &self,
        model_bytes: &[u8],
        inputs: Option<&[Shape<'_>]>,
        outputs: Option<&[Shape<'_>]>,
    ) -> Result<Box<dyn hotg_runicos_base_runtime::Model>, anyhow::Error> {
        let inputs = inputs.context("No input shape was provided")?;
        let outputs = outputs.context("No output shape was provided")?;
        todo!()
    }
}

unsafe impl Send for ModelHandler {}
unsafe impl Sync for ModelHandler {}

#[no_mangle]
pub unsafe extern "C" fn rune_image_register_capability_handler(
    image: *mut Image,
    capability_type: u32,
    user_data: *mut c_void,
    create_capability: unsafe extern "C" fn(*mut c_void),
    free: Option<unsafe extern "C" fn(*mut c_void)>,
) -> *mut Error {
    ensure!(!image.is_null());
    let image = &mut *image;

    let handler = CapabilityHandler {
        user_data,
        create_capability,
        free,
    };
    image.register_capability(capability_type, handler);

    ptr::null_mut()
}

#[repr(C)]
pub struct CapabilityHandler {
    user_data: *mut c_void,
    create_capability: unsafe extern "C" fn(*mut c_void),
    free: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl Drop for CapabilityHandler {
    fn drop(&mut self) {
        if let Some(free) = self.free {
            unsafe {
                free(self.user_data);
            }
        }
    }
}

impl hotg_runicos_base_runtime::CapabilityFactory for CapabilityHandler {
    fn new_capability(
        &self,
    ) -> Result<Box<dyn hotg_rune_runtime::Capability>, anyhow::Error> {
        todo!()
    }
}

unsafe impl Send for CapabilityHandler {}
unsafe impl Sync for CapabilityHandler {}

#[no_mangle]
pub unsafe extern "C" fn rune_image_register_output_handler(
    image: *mut Image,
    output_type: u32,
    user_data: *mut c_void,
    create_output: unsafe extern "C" fn(*mut c_void),
    free: Option<unsafe extern "C" fn(*mut c_void)>,
) -> *mut Error {
    ensure!(!image.is_null());
    let image = &mut *image;

    let handler = OutputHandler {
        user_data,
        create_output,
        free,
    };
    image.register_output(output_type, handler);

    ptr::null_mut()
}

#[repr(C)]
pub struct OutputHandler {
    user_data: *mut c_void,
    create_output: unsafe extern "C" fn(*mut c_void),
    free: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl Drop for OutputHandler {
    fn drop(&mut self) {
        if let Some(free) = self.free {
            unsafe {
                free(self.user_data);
            }
        }
    }
}

impl hotg_runicos_base_runtime::OutputFactory for OutputHandler {
    fn new_output(
        &self,
        inputs: Option<&[Shape<'_>]>,
    ) -> Result<Box<dyn hotg_rune_runtime::Output>, anyhow::Error> {
        todo!()
    }
}

unsafe impl Send for OutputHandler {}
unsafe impl Sync for OutputHandler {}

#[no_mangle]
pub unsafe extern "C" fn rune_image_free(img: *mut Image) {
    if img.is_null() {
        return;
    }

    let _ = Box::from_raw(img);
}
