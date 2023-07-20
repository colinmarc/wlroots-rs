use wayland_server::DisplayHandle;
use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_renderer, wlr_renderer_autocreate, wlr_renderer_init_wl_display};

use crate::{backend::Backend, Destroyable, Handle, WlrError};

pub struct Renderer(Box<Handle<wlr_renderer>>);

impl Renderer {
    pub fn autocreate(backend: &Backend) -> Result<Renderer, WlrError> {
        let backend = backend.handle().as_ptr();
        let ptr = unsafe { wlr_renderer_autocreate(backend).as_mut() };

        match ptr {
            Some(v) => Ok(Self(Handle::new(v))),
            None => Err(WlrError::CallFailed("wlr_renderer_autocreate".into())),
        }
    }

    pub fn init_display(&self, display: &DisplayHandle) -> Result<(), WlrError> {
        let ptr = self.0.as_ptr();

        let display = display.backend_handle().display_ptr();

        unsafe {
            if !wlr_renderer_init_wl_display(ptr, display) {
                return Err(WlrError::CallFailed("wlr_renderer_init_wl_display".into()));
            }
        }

        Ok(())
    }

    pub fn handle(&self) -> &Handle<wlr_renderer> {
        &self.0
    }
}

impl Destroyable for wlr_renderer {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
