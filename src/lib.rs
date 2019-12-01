use wlroots_sys::*;
use wlroots_sys::wayland_sys::server::signal::wl_signal_add;

use std::marker::PhantomPinned;
use std::pin::Pin;

#[macro_use]
mod macros;

#[repr(C)]
pub struct Server {
    display: *mut wl_display,
    backend: *mut wlr_backend,
    renderer: *mut wlr_renderer,

    xdg_shell: *mut wlr_xdg_shell,
    cursor: *mut wlr_cursor,
    cursor_mgr: *mut wlr_xcursor_manager,
    seat: *mut wlr_seat,
    output_layout: *mut wlr_output_layout,

    socket_name: String,

    backend_new_output_listener: wl_listener,
    xdg_shell_new_surface_listener: wl_listener,

    #[allow(dead_code)]
    unpin: PhantomPinned,
}

impl Server {
    pub fn new() -> Result<Pin<Box<Server>>, &'static str> {
        let mut c = Box::pin(Server {
            unpin: PhantomPinned,
            display: std::ptr::null_mut(),
            backend: std::ptr::null_mut(),
            renderer: std::ptr::null_mut(),
            xdg_shell: std::ptr::null_mut(),
            cursor: std::ptr::null_mut(),
            cursor_mgr: std::ptr::null_mut(),
            seat: std::ptr::null_mut(),
            output_layout: std::ptr::null_mut(),
            socket_name: String::new(),
            backend_new_output_listener: unsafe {std::mem::zeroed()},
            xdg_shell_new_surface_listener: unsafe {std::mem::zeroed()},
        });
        unsafe {
            let ctx = c.as_mut().get_unchecked_mut();
            ctx.display =
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_create,) as *mut wl_display;
            ctx.backend = wlr_backend_autocreate(ctx.display, None);
            ctx.renderer = wlr_backend_get_renderer(ctx.backend);
            wlr_renderer_init_wl_display(ctx.renderer, ctx.display);
            wlr_compositor_create(ctx.display, ctx.renderer);
            wlr_data_device_manager_create(ctx.display);

            ctx.output_layout = wlr_output_layout_create();

            connect_listener!(ctx, backend, new_output);

            ctx.xdg_shell = wlr_xdg_shell_create(ctx.display);

            connect_listener!(ctx, xdg_shell, new_surface);

            ctx.cursor = wlr_cursor_create();
            wlr_cursor_attach_output_layout(ctx.cursor, ctx.output_layout);
            ctx.cursor_mgr = wlr_xcursor_manager_create(std::ptr::null(), 24);
            wlr_xcursor_manager_load(ctx.cursor_mgr, 1.0);
            ctx.seat = wlr_seat_create(ctx.display, b"seat0\0".as_ptr() as *const _);
            let socket_name_ptr = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_add_socket_auto, ctx.display as *mut _);
            if socket_name_ptr.is_null() {
                return Err("cannot create socket");
            }
            let socket_name_cstr = std::ffi::CStr::from_ptr(socket_name_ptr);
            ctx.socket_name = socket_name_cstr.to_str().expect("wayland socket name is not utf8").to_owned();

            if !wlr_backend_start(ctx.backend) {
                return Err("cannot start backend");
            }
        }
        Ok(c)
    }
    
    pub fn main_loop(mut self: Pin<&mut Self>) {
        unsafe {
            let ctx = self.as_mut().get_unchecked_mut();
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_run, ctx.display as *mut _);
        }
    }
}

impl std::ops::Drop for Server {
    fn drop(&mut self) {
        // `new_unchecked` is okay because we know this value is never used
        // again after being dropped.
        inner_drop(unsafe { Pin::new_unchecked(self)});
        fn inner_drop(mut this: Pin<&mut Server>) {
            // Actual drop code goes here.
            unsafe {
                let ctx = this.as_mut().get_unchecked_mut();
                wlr_backend_destroy(ctx.backend);
                ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_destroy, ctx.display as *mut _);
            }
        }
    }
}

implement_listener!(Server, backend, new_output, wlr_output);
implement_listener!(Server, xdg_shell, new_surface, wlr_xdg_surface);
impl Server {
    fn backend_new_output(self: Pin<&mut Self>, output: *mut wlr_output) {
        println!("new output!");
    }
    fn xdg_shell_new_surface(self: Pin<&mut Self>, surface: *mut wlr_xdg_surface) {
        println!("new xdg surface!");
    }
}

