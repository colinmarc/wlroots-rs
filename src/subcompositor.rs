use wayland_server::DisplayHandle;
use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_subcompositor, wlr_subcompositor_create};

use crate::{Destroyable, Handle, WlrError};

pub struct Subcompositor(Box<Handle<wlr_subcompositor>>);

impl Subcompositor {
    pub fn new(display: &DisplayHandle) -> Result<Self, WlrError> {
        let display = display.backend_handle().display_ptr();
        let ptr = unsafe { wlr_subcompositor_create(display).as_mut() };

        match ptr {
            Some(v) => Ok(Self(Handle::new(v))),
            None => Err(WlrError::CallFailed("wlr_subcompositor_create".into())),
        }
    }

    pub fn handle(&self) -> &Handle<wlr_subcompositor> {
        &self.0
    }
}

impl Destroyable for wlr_subcompositor {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
