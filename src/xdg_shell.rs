use wayland_server::DisplayHandle;
use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_xdg_shell, wlr_xdg_shell_create, wlr_xdg_surface};

use crate::{Destroyable, Handle, WlrError};

pub struct XdgShell(Box<Handle<wlr_xdg_shell>>);

impl XdgShell {
    pub fn new(display: &DisplayHandle, version: u32) -> Result<Self, WlrError> {
        let ptr = unsafe {
            wlr_xdg_shell_create(display.backend_handle().display_ptr(), version).as_mut()
        };
        match ptr {
            Some(xdg_shell) => Ok(Self(Handle::new(xdg_shell))),
            None => Err(WlrError::CallFailed("wlr_xdg_shell_create".into())),
        }
    }

    pub fn handle(&self) -> &Handle<wlr_xdg_shell> {
        &self.0
    }

    pub fn on_new_surface(&mut self, cb: impl Fn(XdgSurface) + 'static) {
        let signal = unsafe { &mut (*self.handle().as_ptr()).events.new_surface };
        self.0.add_listener(signal, move |data| {
            (cb)(XdgSurface::from_ptr(data as *mut _));
        });
    }
}

impl Destroyable for wlr_xdg_shell {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}

pub enum XdgSurface {
    XdgSurfaceToplevel {
        surf: Box<Handle<wlr_xdg_surface>>,
        toplevel: Box<Handle<wlr_xdg_toplevel>>,
    },
    XdgSurfacePopup(Box<Handle<wlr_xdg_surface>>),
}

pub struct XdgSurfaceToplevel(Box<Handle<wlr_xdg_surface>>);

impl XdgSurface {
    pub fn from_ptr(ptr: *mut wlr_xdg_surface) -> Self {
        unsafe {
            if (*ptr).toplevel.is_some() {
                XdgSurfaceToplevel(Box::new(Handle::new(ptr)))
            }
        }
    }

    pub fn handle(&self) -> &Handle<wlr_xdg_surface> {
        &self.0
    }

    pub fn on_ping_timeout(&mut self, cb: impl Fn() + 'static) {
        let signal = unsafe { &mut (*self.handle().as_ptr()).events.ping_timeout };
        self.0.add_listener(signal, move |_| {
            (cb)();
        });
    }

    pub fn on_new_popup(&mut self, cb: impl Fn(XdgSurface) + 'static) {
        let signal = unsafe { &mut (*self.handle().as_ptr()).events.new_popup };
        self.0.add_listener(signal, move |data| {
            (cb)(XdgSurface::from_ptr(data as *mut _));
        });
    }
}

impl Destroyable for wlr_xdg_surface {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
