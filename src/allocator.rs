use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_allocator, wlr_allocator_autocreate};

use crate::{backend::Backend, renderer::Renderer, Destroyable, Handle, WlrError};

pub struct Allocator(Box<Handle<wlr_allocator>>);

impl Allocator {
    pub fn autocreate(backend: &Backend, renderer: &Renderer) -> Result<Allocator, WlrError> {
        let backend = backend.handle().try_as_ptr()?;
        let ptr = unsafe { wlr_allocator_autocreate(backend, renderer.handle().as_ptr()).as_mut() };

        match ptr {
            Some(v) => Ok(Self(Handle::new(v))),
            None => Err(WlrError::CallFailed("wlr_allocator_autocreate".into())),
        }
    }

    pub fn handle(&self) -> &Handle<wlr_allocator> {
        &self.0
    }
}

impl Destroyable for wlr_allocator {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
