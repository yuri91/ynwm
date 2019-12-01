use wlroots_sys::*;
use wlroots_sys::wayland_sys::server::signal::wl_signal_add;
use wlroots_sys::wayland_server::protocol::wl_seat::Capability;
use wlroots_sys::wlr_log_importance::*;

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

    outputs: Vec<Pin<Box<Output>>>,

    backend_new_output_listener: wl_listener,
    backend_new_input_listener: wl_listener,
    xdg_shell_new_surface_listener: wl_listener,
    cursor_motion_listener: wl_listener,
    cursor_motion_absolute_listener: wl_listener,
    cursor_button_listener: wl_listener,
    cursor_axis_listener: wl_listener,
    cursor_frame_listener: wl_listener,
    seat_request_set_cursor_listener: wl_listener,

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

            outputs: Vec::new(),

            backend_new_output_listener: unsafe {std::mem::zeroed()},
            backend_new_input_listener: unsafe {std::mem::zeroed()},
            xdg_shell_new_surface_listener: unsafe {std::mem::zeroed()},
            cursor_motion_listener: unsafe {std::mem::zeroed()},
            cursor_motion_absolute_listener: unsafe {std::mem::zeroed()},
            cursor_button_listener: unsafe {std::mem::zeroed()},
            cursor_axis_listener: unsafe {std::mem::zeroed()},
            cursor_frame_listener: unsafe {std::mem::zeroed()},
            seat_request_set_cursor_listener: unsafe {std::mem::zeroed()},
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
            connect_listener!(ctx, backend, new_input);

            ctx.xdg_shell = wlr_xdg_shell_create(ctx.display);

            connect_listener!(ctx, xdg_shell, new_surface);

            ctx.cursor = wlr_cursor_create();
            wlr_cursor_attach_output_layout(ctx.cursor, ctx.output_layout);
            ctx.cursor_mgr = wlr_xcursor_manager_create(std::ptr::null(), 24);
            wlr_xcursor_manager_load(ctx.cursor_mgr, 1.0);


            connect_listener!(ctx, cursor, motion);
            connect_listener!(ctx, cursor, motion_absolute);
            connect_listener!(ctx, cursor, button);
            connect_listener!(ctx, cursor, axis);
            connect_listener!(ctx, cursor, frame);

            ctx.seat = wlr_seat_create(ctx.display, b"seat0\0".as_ptr() as *const _);

            connect_listener!(ctx, seat, request_set_cursor);

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
implement_listener!(Server, backend, new_input, wlr_input_device);
implement_listener!(Server, xdg_shell, new_surface, wlr_xdg_surface);
implement_listener!(Server, cursor, motion, wlr_event_pointer_motion);
implement_listener!(Server, cursor, motion_absolute, wlr_event_pointer_motion_absolute);
implement_listener!(Server, cursor, button, wlr_event_pointer_button);
implement_listener!(Server, cursor, axis, wlr_event_pointer_axis);
implement_listener!(Server, cursor, frame, libc::c_void);
implement_listener!(Server, seat, request_set_cursor, wlr_seat_pointer_request_set_cursor_event);
impl Server {
    fn backend_new_output(self: Pin<&mut Self>, output_ptr: *mut wlr_output) {
        wlr_log!(WLR_INFO,"new output!");

        let output = Output::new(&self.as_ref(), output_ptr);
        unsafe {
            // check that list is not empty
            if (*output_ptr).modes.next != &(*output_ptr).modes as *const _ as *mut _ {
                let mode = container_of!((*output_ptr).modes.prev, wlr_output_mode, link);
                wlr_output_set_mode(output_ptr, mode);
            }
            wlr_output_layout_add_auto(self.as_ref().output_layout, output_ptr);
        }

        let ctx = unsafe {self.get_unchecked_mut()};
        ctx.outputs.push(output);
    }
    fn backend_new_input(self: Pin<&mut Self>, input_ptr: *mut wlr_input_device) {
        // UNSAFE: promise that we will not move the value out of ctx
        let ctx = unsafe {self.get_unchecked_mut()};
        let input = unsafe {&*input_ptr};
        match input.type_ {
            wlr_input_device_type::WLR_INPUT_DEVICE_POINTER => {
                unsafe {
                    wlr_cursor_attach_input_device(ctx.cursor, input_ptr);
                }
            },
            wlr_input_device_type::WLR_INPUT_DEVICE_KEYBOARD => {
            },
            _ => {
            }
        }

        let caps = Capability::Pointer;
        unsafe {
            wlr_seat_set_capabilities(ctx.seat, caps.to_raw());
        }

    }
    fn xdg_shell_new_surface(self: Pin<&mut Self>, surface: *mut wlr_xdg_surface) {
        println!("new xdg surface!");
    }
    fn cursor_motion(self: Pin<&mut Self>, event: *mut wlr_event_pointer_motion) {
        println!("cursor motion!");
    }
    fn cursor_motion_absolute(self: Pin<&mut Self>, event: *mut wlr_event_pointer_motion_absolute) {
        println!("cursor motion absolute!");
    }
    fn cursor_button(self: Pin<&mut Self>, event: *mut wlr_event_pointer_button) {
        println!("cursor button!");
    }
    fn cursor_axis(self: Pin<&mut Self>, event: *mut wlr_event_pointer_axis) {
        println!("cursor axis!");
    }
    fn cursor_frame(self: Pin<&mut Self>, event: *mut libc::c_void) {
        println!("cursor motion!");
    }
    fn seat_request_set_cursor(self: Pin<&mut Self>, event: *mut wlr_seat_pointer_request_set_cursor_event) {
        println!("request set cursor!");
    }
}

#[repr(C)]
pub struct Output {
    server: *mut Server,
    output: *mut wlr_output,

    output_frame_listener: wl_listener,
}

impl Output {
    pub fn new(server: &Server, output: *mut wlr_output) -> Pin<Box<Output>> {
        let o = Output {
            server: server as *const _ as *mut _,
            output,
            output_frame_listener: unsafe{std::mem::zeroed()},
        };
        let mut o = Box::pin(o);

        unsafe {
            let ctx = o.as_mut().get_unchecked_mut();

            connect_listener!(ctx, output, frame);

            wlr_output_create_global(output);
        }

        o
    }
}

implement_listener!(Output, output, frame, libc::c_void);
impl Output {
    fn output_frame(self: Pin<&mut Self>, _: *mut libc::c_void) {
    }
}
