use wayland_server::DisplayHandle;
use wayland_sys::server::wl_signal;
use wlroots_sys::{wlr_data_device_manager, wlr_data_device_manager_create};

use crate::{Destroyable, Handle, WlrError};

pub struct DataDeviceManager(Box<Handle<wlr_data_device_manager>>);

impl DataDeviceManager {
    pub fn new(display: &DisplayHandle) -> Result<Self, WlrError> {
        let display = display.backend_handle().display_ptr();
        let ptr = unsafe { wlr_data_device_manager_create(display).as_mut() };

        match ptr {
            Some(v) => Ok(Self(Handle::new(v))),
            None => Err(WlrError::CallFailed(
                "wlr_data_device_manager_create".into(),
            )),
        }
    }

    pub fn handle(&self) -> &Handle<wlr_data_device_manager> {
        &self.0
    }
}

impl Destroyable for wlr_data_device_manager {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}
