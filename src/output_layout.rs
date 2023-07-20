use wayland_sys::server::wl_signal;
use wlroots_sys::{
    wlr_output_layout, wlr_output_layout_add_auto, wlr_output_layout_create,
    wlr_output_layout_destroy,
};

use crate::{Destroyable, Handle, Output, WlrError};

pub struct OutputLayout(Box<Handle<wlr_output_layout>>);

impl OutputLayout {
    pub fn new() -> Result<Self, WlrError> {
        let ptr = unsafe { wlr_output_layout_create().as_mut() };
        match ptr {
            Some(layout) => Ok(Self(Handle::new(layout))),
            None => Err(WlrError::CallFailed("wlr_output_layout_create".into())),
        }
    }

    pub fn add_auto(&self, output: &Output) {
        unsafe {
            wlr_output_layout_add_auto(self.0.as_ptr(), output.handle().as_ptr());
        }
    }

    pub fn handle(&self) -> &Handle<wlr_output_layout> {
        &self.0
    }
}

impl Drop for OutputLayout {
    fn drop(&mut self) {
        unsafe { wlr_output_layout_destroy(self.0.as_ptr()) }
    }
}

impl Destroyable for wlr_output_layout {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
