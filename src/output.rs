use wayland_sys::{common::wl_list, server::wl_signal};
use wlroots_sys::{
    wlr_output, wlr_output_commit, wlr_output_enable, wlr_output_init_render, wlr_output_mode,
    wlr_output_preferred_mode, wlr_output_set_mode,
};

use crate::{macros::*, Allocator, ChildHandle, Destroyable, Handle, Renderer, WlrError};

pub struct Output(Box<Handle<wlr_output>>);

impl Output {
    pub fn from_ptr(ptr: *mut wlr_output) -> Self {
        Self(Handle::new(ptr))
    }

    pub fn handle(&self) -> &Handle<wlr_output> {
        &self.0
    }

    pub fn modes(&self) -> impl Iterator<Item = OutputMode> {
        let ptr = self.0.as_ptr();
        let head = unsafe { &mut (*ptr).modes as *mut wl_list };
        OutputModeIterator {
            current: head,
            head: head,
            parent: &self.0,
        }
    }

    pub fn preferred_mode(&self) -> Option<OutputMode> {
        let ptr = self.0.as_ptr();

        unsafe {
            match wlr_output_preferred_mode(ptr).as_mut() {
                Some(mode) => Some(OutputMode(ChildHandle::new(mode, self.handle()))),
                None => None,
            }
        }
    }

    pub fn set_mode(&self, mode: OutputMode) {
        unsafe { wlr_output_set_mode(self.0.as_ptr(), mode.0.as_ptr()) }
    }

    pub fn enable(&self, enable: bool) {
        unsafe { wlr_output_enable(self.0.as_ptr(), enable) }
    }

    pub fn commit(&self) -> Result<(), WlrError> {
        unsafe {
            if !wlr_output_commit(self.0.as_ptr()) {
                Err(WlrError::CallFailed("wlr_output_commit".into()))
            } else {
                Ok(())
            }
        }
    }

    pub fn init_render(
        &mut self,
        allocator: &Allocator,
        renderer: &Renderer,
    ) -> Result<(), WlrError> {
        let output = self.0.try_as_ptr()?;
        let allocator = allocator.handle().try_as_ptr()?;
        let renderer = renderer.handle().try_as_ptr()?;
        unsafe {
            if !wlr_output_init_render(output, allocator, renderer) {
                return Err(WlrError::CallFailed("wlr_output_init_render".into()));
            }
        }

        Ok(())
    }

    pub fn on_frame(&mut self, cb: impl Fn() + 'static) {
        let signal = unsafe { &mut (*self.handle().as_ptr()).events.frame };
        self.0.add_listener(signal, move |_data| {
            (cb)();
        });
    }
}

impl Destroyable for wlr_output {
    fn destroy_signal(&mut self) -> *mut wl_signal {
        &mut self.events.destroy
    }
}

pub struct OutputMode<'parent>(ChildHandle<'parent, wlr_output_mode, wlr_output>);

impl OutputMode<'_> {
    pub fn dimensions(&self) -> (i32, i32) {
        let p = self.0.as_ptr();
        unsafe { ((*p).width, (*p).height) }
    }

    pub fn refresh(&self) -> i32 {
        unsafe { (*self.0.as_ptr()).refresh }
    }

    pub fn preferred(&self) -> bool {
        unsafe { (*self.0.as_ptr()).preferred }
    }

    pub fn picture_aspect_ratio(&self) -> (i32, i32) {
        unsafe { ((*self.0.as_ptr()).width, (*self.0.as_ptr()).height) }
    }
}

struct OutputModeIterator<'parent> {
    current: *mut wl_list,
    head: *mut wl_list,
    parent: &'parent Handle<wlr_output>,
}

impl<'parent> Iterator for OutputModeIterator<'parent> {
    type Item = OutputMode<'parent>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check that the parent is still valid.
        let _ = self.parent.as_ptr();

        unsafe {
            if (*self.current).next == self.head {
                None
            } else {
                let mode = container_of!((*self.current).next, wlr_output_mode, link)
                    as *mut wlr_output_mode;
                self.current = (*mode).link.next;
                Some(OutputMode(ChildHandle::new(mode, self.parent)))
            }
        }
    }
}
