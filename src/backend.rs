use wayland_server::DisplayHandle;
use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_backend, wlr_backend_autocreate, wlr_backend_destroy, wlr_output};

use crate::{output::Output, Destroyable, Handle, WlrError};

pub struct Backend(Box<Handle<wlr_backend>>);

impl Backend {
    pub fn autocreate(display: &DisplayHandle) -> Result<Backend, WlrError> {
        let ptr =
            unsafe { wlr_backend_autocreate(display.backend_handle().display_ptr()).as_mut() };
        match ptr {
            Some(backend) => Ok(Self(Handle::new(backend))),
            None => Err(WlrError::CallFailed("wlr_backend_autocreate".into())),
        }
    }

    pub fn from_ptr(ptr: *mut wlr_backend) -> Self {
        Self(Handle::new(ptr))
    }

    pub fn handle(&self) -> &Handle<wlr_backend> {
        &self.0
    }

    pub fn on_new_output(&mut self, cb: impl Fn(Output) + 'static) {
        let signal = unsafe { &mut (*self.handle().as_ptr()).events.new_output };
        self.0.add_listener(signal, move |data| {
            (cb)(Output::from_ptr(data as *mut wlr_output));
        });
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        unsafe { wlr_backend_destroy(self.handle().as_ptr()) };
    }
}

impl Destroyable for wlr_backend {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
