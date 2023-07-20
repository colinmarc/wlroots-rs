use wayland_server::DisplayHandle;
use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_compositor, wlr_compositor_create};

use crate::{Destroyable, Handle, Renderer, WlrError};

pub struct Compositor(Box<Handle<wlr_compositor>>);

impl Compositor {
    pub fn new(display: &DisplayHandle, renderer: &Renderer) -> Result<Self, WlrError> {
        let display = display.backend_handle().display_ptr();
        let ptr = unsafe { wlr_compositor_create(display, renderer.handle().as_ptr()).as_mut() };

        match ptr {
            Some(v) => Ok(Self(Handle::new(v))),
            None => Err(WlrError::CallFailed("wlr_compositor_create".into())),
        }
    }

    pub fn handle(&self) -> &Handle<wlr_compositor> {
        &self.0
    }
}

impl Destroyable for wlr_compositor {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
