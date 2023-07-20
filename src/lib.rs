mod allocator;
mod backend;
mod compositor;
mod data_device_manager;
mod output;
mod output_layout;
mod renderer;
mod scene;
mod subcompositor;
mod xdg_shell;

pub use allocator::Allocator;
pub use backend::Backend;
pub use compositor::Compositor;
pub use data_device_manager::DataDeviceManager;
pub use output::{Output, OutputMode};
pub use output_layout::OutputLayout;
pub use renderer::Renderer;
pub use scene::Scene;
pub use subcompositor::Subcompositor;
pub use xdg_shell::XdgShell;

use std::{any::type_name, os::raw::c_void};
use thin_trait_object::*;

use wayland_sys::server::{
    signal::{
        rust_listener_create, rust_listener_destroy, rust_listener_get_user_data,
        rust_listener_set_user_data, wl_signal_add,
    },
    wl_list_remove, wl_listener, wl_signal,
};

/// A trait for wlroots objects that have a destroy callback.
pub trait Destroyable {
    fn destroy_signal(&mut self) -> *mut wl_signal;
}

/// A weak reference to a wlroots object. Memory is generally managed by wlroots
/// itself, which means that the library allocates and destroys memory,
/// "borrowing" it to rust in the meantime. In C, user code is notified by a
/// callback just before the object is free'd, to let the user remove any
/// dangling references.
///
/// The rust code attains a measure of memory safety by tracking that destroy
/// callback, and doing a runtime check to ensure that the callback has not yet
/// been called before deferencing the underlying pointer.
///
/// Technically, this means that any operation on a Handle can panic; in normal
/// usage, this is unlikely to happen.
pub struct Handle<T: Destroyable> {
    ptr: Option<*mut T>,
    listeners: Vec<*mut wl_listener>,
}

impl<T: Destroyable> Handle<T> {
    pub fn new(ptr: *mut T) -> Box<Self> {
        // We return a box so that the handle is findable by callbacks.
        let mut handle = Box::new(Self {
            ptr: Some(ptr),
            listeners: Vec::new(),
        });

        // Attach a callback to the destroy handle; if it fires, we know the
        // underlying object is about to be freed.
        let signal = unsafe { (*ptr).destroy_signal() };
        let listener = rust_listener_create(generic_destroy_callback::<T>);
        unsafe {
            rust_listener_set_user_data(listener, &mut *handle as *mut _ as *mut c_void);
            wl_signal_add(signal, listener);
        }

        handle
    }

    /// Returns the underlying pointer to the wlroots object. This will panic
    /// if the object has since been destroyed. Holding on to the pointer while
    /// control is passed back to wlroots is unsafe.
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
            .expect(format!("operation on destroyed {}", type_name::<T>()).as_str())
    }

    /// Returns the underlying pointer to the wlroots object.
    pub fn try_as_ptr(&self) -> Result<*mut T, WlrError> {
        self.ptr
            .ok_or(WlrError::ObjectDestroyed(type_name::<T>().into()))
    }

    fn add_listener(&mut self, signal: *mut wl_signal, callback: impl Callback) {
        let callback = BoxedCallback::new(callback);
        let listener = rust_listener_create(generic_callback);
        self.listeners.push(listener);

        unsafe {
            rust_listener_set_user_data(listener, callback.into_raw() as *mut c_void);
            wl_signal_add(signal, listener);
        }
    }

    fn on_destroy(&mut self) {
        self.ptr = None;
        self.cleanup_listeners();
    }

    fn cleanup_listeners(&mut self) {
        // Remove the listeners from their respective signals.
        for listener in self.listeners.drain(..) {
            // SAFETY: the listeners were created by us in add_listener.
            unsafe {
                wl_list_remove(&mut (*listener).link);

                // TODO: ???
                // let _closure = Box::from_raw(rust_listener_get_user_data(listener)
                //     as *mut dyn Fn(*mut c_void)
                //     as &mut _);
                rust_listener_destroy(listener);
            }
        }
    }
}

impl<T: Destroyable> Drop for Handle<T> {
    fn drop(&mut self) {
        // Dropping the handle doesn't necessarily mean the underlying wlr_foo
        // is freed. Most objects are managed by wlroots itself. However, we
        // should remove and drop any rust listeners.
        self.cleanup_listeners();
    }
}

/// Like a Handle, but with a borrowed refeference to a parent object.
pub struct ChildHandle<'parent, T, P: Destroyable> {
    ptr: *mut T,
    parent: &'parent Handle<P>,
}

impl<'parent, T, P: Destroyable> ChildHandle<'parent, T, P> {
    pub fn new(ptr: *mut T, parent: &'parent Handle<P>) -> Self {
        Self { ptr, parent }
    }

    /// Returns the underlying pointer, checking that the parent pointer is
    /// still valid. Holding on to the pointer while control is passed back to
    /// wlroots is unsafe.
    pub fn as_ptr(&self) -> *mut T {
        // Make sure the parent is still valid.
        let _ = self.parent.as_ptr();

        self.ptr
    }

    pub fn try_as_ptr(&self) -> Result<*mut T, WlrError> {
        // Make sure the parent is still valid.
        let _ = self.parent.try_as_ptr()?;

        Ok(self.ptr)
    }
}

#[thin_trait_object]
trait Callback {
    fn call(&self, data: *mut c_void);
}

impl<F> Callback for F
where
    F: Fn(*mut c_void) + 'static,
{
    fn call(&self, data: *mut c_void) {
        self(data)
    }
}

unsafe extern "C" fn generic_callback(listener: *mut wl_listener, data: *mut c_void) {
    let callback = BoxedCallback::from_raw(rust_listener_get_user_data(listener) as *mut ());
    callback.call(data);

    // We'll drop the callback when we deregister it. Leave it on the heap for now.
    std::mem::forget(callback);
}

unsafe extern "C" fn generic_destroy_callback<T: Destroyable>(
    listener: *mut wl_listener,
    _data: *mut c_void,
) {
    let handle = rust_listener_get_user_data(listener) as *mut Handle<T>;
    handle.as_mut().unwrap().on_destroy();

    // We don't need the listener anymore.
    wl_list_remove(&mut (*listener).link);
    rust_listener_destroy(listener);
}

#[derive(thiserror::Error, Debug)]
pub enum WlrError {
    #[error("use of destroyed {0}")]
    ObjectDestroyed(String),
    #[error("call to {0} failed")]
    CallFailed(String),
}

pub(crate) mod macros {
    // Stolen from wayland-rs.
    macro_rules! container_of(
        ($ptr: expr, $container: ident, $field: ident) => {
            ($ptr as *mut u8).sub(memoffset::offset_of!($container, $field)) as *mut $container
        }
    );

    macro_rules! list_for_each(
        ($pos: ident, $head:expr, $container: ident, $field: ident, $action: block) => {
            let mut $pos = container_of!((*$head).next, $container, $field);
            while &mut (*$pos).$field as *mut _ != $head {
                $action;
                $pos = container_of!((*$pos).$field.next, $container, $field);
            }
        }
    );

    pub(crate) use container_of;
    pub(crate) use list_for_each;
}
