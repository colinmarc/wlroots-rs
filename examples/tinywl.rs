use std::{cell::RefCell, rc::Rc};

use wayland_server::Display;
use wayland_sys::server::*;
use wlroots_sys::*;

use anyhow::Result;
use wlroots::{
    Allocator, Backend, Compositor, DataDeviceManager, Output, OutputLayout, Renderer, Scene,
    Subcompositor, XdgShell,
};

struct State {
    backend: Backend,
    renderer: Renderer,
    allocator: Allocator,
    output_layout: OutputLayout,
    xdg_shell: XdgShell,
    outputs: Vec<Output>,
}

fn main() -> Result<()> {
    unsafe {
        wlr_log_init(wlr_log_importance_WLR_DEBUG, None);
    }

    let display: Display<State> = Display::new().expect("failed to create display");
    let display_handle = display.handle();

    let backend = Backend::autocreate(&display_handle)?;
    let backend_ptr = backend.handle().as_ptr();

    let renderer = Renderer::autocreate(&backend)?;
    renderer.init_display(&display_handle)?;

    let allocator = Allocator::autocreate(&backend, &renderer)?;
    let output_layout = OutputLayout::new()?;

    let _ = Compositor::new(&display_handle, &renderer)?;
    let _ = Subcompositor::new(&display_handle)?;
    let _ = DataDeviceManager::new(&display_handle)?;

    let scene = Scene::new()?;
    scene.attach_output_layout(&output_layout);

    let xdg_shell = XdgShell::new(&display_handle, 3);

    let outputs = Vec::new();

    let state: Rc<RefCell<State>> = Rc::new(RefCell::new(State {
        backend,
        renderer,
        allocator,
        output_layout,
        xdg_shell,
        outputs,
    }));

    let state_clone = state.clone();
    state.borrow_mut().backend.on_new_output(move |mut output| {
        eprintln!("new output!");
        let mut state = state_clone.borrow_mut();

        output
            .init_render(&state.allocator, &state.renderer)
            .expect("failed to init renderer");

        for mode in output.modes() {
            eprintln!("mode: {:?}", mode.dimensions());
        }

        if let Some(mode) = output.preferred_mode() {
            output.set_mode(mode);
            output.enable(true);
            output.commit().expect("initial commit failed");
        }

        output.on_frame(|| eprintln!("frame!"));

        state.output_layout.add_auto(&output);

        state.outputs.push(output);
    });

    state.borrow_mut().xdg_shell.on_new_surface(|surface| {
        eprintln!("new surface!");

        match surface {
            XdgSurface::Popup(popup) => {
                let parent = XdgSurface::from_surface(popup.parent());
                // TODO: ergonomics?
                let tree = state.scene.create_xdg_surface(parent.data(), &surface);
                surface.set_userdata(tree);
                return;
            }
        }

        // TODO: many event handlers.
    });

    unsafe {
        // Start the backend.
        if !wlr_backend_start(backend_ptr) {
            wl_display_destroy(display_handle.backend_handle().display_ptr());
            panic!("failed to start backend")
        }

        wl_display_run(display_handle.backend_handle().display_ptr());
        eprintln!("wl_display_run exited")
    }

    Ok(())
}
