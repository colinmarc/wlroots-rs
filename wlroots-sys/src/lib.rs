#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libc;
extern crate libudev_sys;
extern crate wayland_sys;

use libc::{clockid_t, dev_t, off_t, timespec};
use libudev_sys::{udev, udev_monitor};
use wayland_sys::common::*;
use wayland_sys::server::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
