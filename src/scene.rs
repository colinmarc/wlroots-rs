use libc::c_void;
use wlroots_sys::{wlr_scene, wlr_scene_attach_output_layout, wlr_scene_create};

use crate::{OutputLayout, WlrError};

pub struct Scene(*mut wlr_scene);

impl Scene {
    pub fn new() -> Result<Self, WlrError> {
        let ptr = unsafe { wlr_scene_create().as_mut() };
        match ptr {
            Some(layout) => Ok(Self(layout)),
            None => Err(WlrError::CallFailed("wlr_scene_create".into())),
        }
    }

    pub fn attach_output_layout(&self, output_layout: &OutputLayout) {
        let output_layout = output_layout.handle().as_ptr();
        unsafe {
            wlr_scene_attach_output_layout(self.0, output_layout);
        }
    }

    pub fn as_ptr(&self) -> *mut wlr_scene {
        self.0
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe { libc::free(self.0 as *mut c_void) };
    }
}
