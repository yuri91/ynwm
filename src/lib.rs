use wlroots_sys::wayland_server::protocol::wl_seat::Capability;
use wlroots_sys::wayland_sys::server::signal::wl_signal_add;
use wlroots_sys::wlr_log_importance::*;
use wlroots_sys::*;

use generational_arena::{Arena, Index};

use std::marker::PhantomPinned;
use std::pin::Pin;

use std::collections::VecDeque;

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

    outputs: Arena<Pin<Box<Output>>>,
    views: Arena<Pin<Box<View>>>,

    dead_views: Vec<Pin<Box<View>>>,

    event_queue: VecDeque<Event>,

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

            outputs: Arena::new(),
            views: Arena::new(),

            dead_views: Vec::new(),

            event_queue: VecDeque::new(),

            backend_new_output_listener: unsafe { std::mem::zeroed() },
            backend_new_input_listener: unsafe { std::mem::zeroed() },
            xdg_shell_new_surface_listener: unsafe { std::mem::zeroed() },
            cursor_motion_listener: unsafe { std::mem::zeroed() },
            cursor_motion_absolute_listener: unsafe { std::mem::zeroed() },
            cursor_button_listener: unsafe { std::mem::zeroed() },
            cursor_axis_listener: unsafe { std::mem::zeroed() },
            cursor_frame_listener: unsafe { std::mem::zeroed() },
            seat_request_set_cursor_listener: unsafe { std::mem::zeroed() },
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

            let socket_name_ptr = ffi_dispatch!(
                WAYLAND_SERVER_HANDLE,
                wl_display_add_socket_auto,
                ctx.display as *mut _
            );
            if socket_name_ptr.is_null() {
                return Err("cannot create socket");
            }
            let socket_name_cstr = std::ffi::CStr::from_ptr(socket_name_ptr);
            ctx.socket_name = socket_name_cstr
                .to_str()
                .expect("wayland socket name is not utf8")
                .to_owned();

            if !wlr_backend_start(ctx.backend) {
                return Err("cannot start backend");
            }
        }
        Ok(c)
    }

    pub fn poll_events(mut self: Pin<&mut Self>) -> impl Iterator<Item=Event> {
        unsafe {
            let ctx = self.as_mut().get_unchecked_mut();
            ctx.dead_views.clear();
            let el = ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_get_event_loop, ctx.display as *mut _) as *mut wayland_sys::server::wl_event_loop;
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_display_flush_clients, ctx.display as *mut _);
            ffi_dispatch!(WAYLAND_SERVER_HANDLE, wl_event_loop_dispatch, el, -1);
            let mut events = VecDeque::new();
            std::mem::swap(&mut events, &mut ctx.event_queue);
            events.into_iter()
        }
    }
    pub fn get_output<'a>(self: Pin<&'a mut Self>, idx: Index) -> Pin<&'a mut Output> {
        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.outputs[idx].as_mut()
    }
}

impl std::ops::Drop for Server {
    fn drop(&mut self) {
        // `new_unchecked` is okay because we know this value is never used
        // again after being dropped.
        inner_drop(unsafe { Pin::new_unchecked(self) });
        fn inner_drop(mut this: Pin<&mut Server>) {
            // Actual drop code goes here.
            unsafe {
                let ctx = this.as_mut().get_unchecked_mut();
                wlr_backend_destroy(ctx.backend);
                ffi_dispatch!(
                    WAYLAND_SERVER_HANDLE,
                    wl_display_destroy,
                    ctx.display as *mut _
                );
            }
        }
    }
}

implement_listener!(Server, backend, new_output, wlr_output);
implement_listener!(Server, backend, new_input, wlr_input_device);
implement_listener!(Server, xdg_shell, new_surface, wlr_xdg_surface);
implement_listener!(Server, cursor, motion, wlr_event_pointer_motion);
implement_listener!(
    Server,
    cursor,
    motion_absolute,
    wlr_event_pointer_motion_absolute
);
implement_listener!(Server, cursor, button, wlr_event_pointer_button);
implement_listener!(Server, cursor, axis, wlr_event_pointer_axis);
implement_listener!(Server, cursor, frame, libc::c_void);
implement_listener!(
    Server,
    seat,
    request_set_cursor,
    wlr_seat_pointer_request_set_cursor_event
);
impl Server {
    fn backend_new_output(self: Pin<&mut Self>, output_ptr: *mut wlr_output) {
        wlr_log!(WLR_INFO, "new output!");

        let output = Output::new(&self.as_ref(), output_ptr);
        unsafe {
            // check that list is not empty
            if (*output_ptr).modes.next != &(*output_ptr).modes as *const _ as *mut _ {
                let mode = container_of!((*output_ptr).modes.prev, wlr_output_mode, link);
                wlr_output_set_mode(output_ptr, mode);
            }
            wlr_output_layout_add_auto(self.as_ref().output_layout, output_ptr);
        }

        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.outputs.insert(output);
    }
    fn backend_new_input(self: Pin<&mut Self>, input_ptr: *mut wlr_input_device) {
        // UNSAFE: promise that we will not move the value out of ctx
        let ctx = unsafe { self.get_unchecked_mut() };
        let input = unsafe { &*input_ptr };
        match input.type_ {
            wlr_input_device_type::WLR_INPUT_DEVICE_POINTER => unsafe {
                wlr_cursor_attach_input_device(ctx.cursor, input_ptr);
            },
            wlr_input_device_type::WLR_INPUT_DEVICE_KEYBOARD => {}
            _ => {}
        }

        let caps = Capability::Pointer;
        unsafe {
            wlr_seat_set_capabilities(ctx.seat, caps.to_raw());
        }
    }
    fn xdg_shell_new_surface(self: Pin<&mut Self>, surface_ptr: *mut wlr_xdg_surface) {
        println!("new xdg surface!");

        unsafe {
            if (*surface_ptr).role != wlr_xdg_surface_role::WLR_XDG_SURFACE_ROLE_TOPLEVEL {
                return;
            }
        }

        let view = View::new(&self.as_ref(), surface_ptr);

        let ctx = unsafe { self.get_unchecked_mut() };
        let idx = ctx.views.insert(view);
        ctx.event_queue.push_back(Event::XdgSurfaceNew {
            view: idx,
        });
    }
    fn cursor_motion(self: Pin<&mut Self>, event: *mut wlr_event_pointer_motion) {
        let e = unsafe { &*(event) };
        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.event_queue.push_back(
            Event::CursorMotion {
                time_ms: e.time_msec,
                delta_x: e.delta_x,
                delta_y: e.delta_y,
            }
        );
    }
    fn cursor_motion_absolute(self: Pin<&mut Self>, event: *mut wlr_event_pointer_motion_absolute) {
        let e = unsafe { &*(event) };
        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.event_queue.push_back(
            Event::CursorMotionAbsolute {
                time_ms: e.time_msec,
                x: e.x,
                y: e.y,
            }
        );
    }
    fn cursor_button(self: Pin<&mut Self>, event: *mut wlr_event_pointer_button) {
        let e = unsafe { &*(event) };
        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.event_queue.push_back(
            Event::CursorButton {
                time_ms: e.time_msec,
                state: e.state,
                button: e.button,
            }
        );
    }
    fn cursor_axis(self: Pin<&mut Self>, event: *mut wlr_event_pointer_axis) {
        let e = unsafe { &*(event) };
        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.event_queue.push_back(
            Event::CursorAxis {
                time_ms: e.time_msec,
                orientation: e.orientation,
                source: e.source,
                delta: e.delta,
                delta_discrete: e.delta_discrete,
            }
        );
    }
    fn cursor_frame(self: Pin<&mut Self>, _: *mut libc::c_void) {
        let ctx = unsafe { self.get_unchecked_mut() };
        ctx.event_queue.push_back(
            Event::CursorFrame
        );
    }
    fn seat_request_set_cursor(
        self: Pin<&mut Self>,
        event: *mut wlr_seat_pointer_request_set_cursor_event,
    ) {
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
            output_frame_listener: unsafe { std::mem::zeroed() },
        };
        let mut o = Box::pin(o);

        unsafe {
            let ctx = o.as_mut().get_unchecked_mut();

            connect_listener!(ctx, output, frame);

            wlr_output_create_global(output);
        }

        o
    }
    pub fn render_views(self: Pin<&mut Self>, views: impl Iterator<Item=(Index, Rect)>) {
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let renderer = server.renderer;

        unsafe {
            if !wlr_output_attach_render(ctx.output, std::ptr::null_mut()) {
                return;
            }
            let mut w: i32 = 0;
            let mut h: i32 = 0;
            wlr_output_effective_resolution(ctx.output, &mut w as *mut _, &mut h as *mut _);
            wlr_renderer_begin(renderer, w, h);
            let color = [0.3, 0.3, 0.3, 1.0];
            wlr_renderer_clear(renderer, color.as_ptr());

            struct CbData {
                r: Rect,
                o: *mut wlr_output,
                ol: *mut wlr_output_layout,
                rend: *mut wlr_renderer,
                when: timespec,
            };
            unsafe extern "C" fn surface_cb(
                surface: *mut wlr_surface,
                sx: i32,
                sy: i32,
                data: *mut libc::c_void
            ) {
                let data = &*(data as *mut CbData);
                let tex = wlr_surface_get_texture(surface);
                if tex.is_null() {
                    return;
                }
                let mut ox: f64 = 0.;
                let mut oy: f64 = 0.;
                wlr_output_layout_output_coords(data.ol, data.o, &mut ox as *mut _, &mut oy as *mut _);
                ox += (data.r.x + sx) as f64;
                oy += (data.r.y + sy) as f64;

                let scale = (*data.o).scale as f64;
                let wbox = wlr_box {
                    x: (ox * scale) as i32,
                    y: (oy * scale) as i32,
                    width: ((*surface).current.width as f64 * scale) as i32,
                    height: ((*surface).current.height as f64 * scale) as i32,
                };
                let mut matrix = [0.0f32; 9];
                let trans = wlr_output_transform_invert((*surface).current.transform);
                wlr_matrix_project_box(matrix.as_mut_ptr(), (&wbox) as *const _, trans, 0., (*data.o).transform_matrix.as_ptr());
                wlr_render_texture_with_matrix(data.rend, tex, matrix.as_ptr(), 1.);
                wlr_surface_send_frame_done(surface, &data.when as *const _);
            }
            for (idx, r) in views {
                let view = server.views[idx].as_ref();
                let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Time travel");
                let when = timespec {
                    tv_sec: when.as_secs() as i64,
                    tv_nsec: when.subsec_nanos() as i64,
                };
                let mut data = CbData {
                    r: r,
                    o: ctx.output,
                    ol: server.output_layout,
                    rend: renderer,
                    when,
                };
                wlr_xdg_surface_for_each_surface(view.xdg_surface, Some(surface_cb), &mut data as *mut _ as *mut _);

            }

            wlr_output_render_software_cursors(ctx.output, std::ptr::null_mut());
            wlr_renderer_end(renderer);
            wlr_output_commit(ctx.output);
        }
    }
}

implement_listener!(Output, output, frame, libc::c_void);
impl Output {
    fn output_frame(self: Pin<&mut Self>, _: *mut libc::c_void) {
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let (index, _) = server.outputs.iter().find(|&(_, o)| {
            o.as_ref().get_ref() as *const _ == ctx as *const _
        }).expect("cant find output in arena");
        server.event_queue.push_back(
            Event::OutputFrame {
                output: index,
                when: std::time::Instant::now(),
            }
        );
    }
}

#[repr(C)]
pub struct View {
    server: *mut Server,
    xdg_surface: *mut wlr_xdg_surface,

    xdg_surface_map_listener: wl_listener,
    xdg_surface_unmap_listener: wl_listener,
    xdg_surface_destroy_listener: wl_listener,
    xdg_surface_request_move_listener: wl_listener,
    xdg_surface_request_resize_listener: wl_listener,
}

impl View {
    pub fn new(server: &Server, xdg_surface: *mut wlr_xdg_surface) -> Pin<Box<View>> {
        let v = View {
            server: server as *const _ as *mut _,
            xdg_surface,

            xdg_surface_map_listener: unsafe { std::mem::zeroed() },
            xdg_surface_unmap_listener: unsafe { std::mem::zeroed() },
            xdg_surface_destroy_listener: unsafe { std::mem::zeroed() },
            xdg_surface_request_move_listener: unsafe { std::mem::zeroed() },
            xdg_surface_request_resize_listener: unsafe { std::mem::zeroed() },
        };
        let mut v = Box::pin(v);

        unsafe {
            let ctx = v.as_mut().get_unchecked_mut();

            connect_listener!(ctx, xdg_surface, map);
            connect_listener!(ctx, xdg_surface, unmap);
            connect_listener!(ctx, xdg_surface, destroy);

            let toplevel =
                (*ctx.xdg_surface).__bindgen_anon_1.toplevel as *const _ as *mut wlr_xdg_toplevel;
            let toplevel = &mut (*toplevel);
            connect_listener!(ctx, toplevel, xdg_surface, request_move);
            connect_listener!(ctx, toplevel, xdg_surface, request_resize);
        }

        v
    }
}

implement_listener!(View, xdg_surface, map, libc::c_void);
implement_listener!(View, xdg_surface, unmap, libc::c_void);
implement_listener!(View, xdg_surface, destroy, libc::c_void);
implement_listener!(View, xdg_surface, request_move, libc::c_void);
implement_listener!(
    View,
    xdg_surface,
    request_resize,
    wlr_xdg_toplevel_resize_event
);
impl View {
    fn xdg_surface_map(self: Pin<&mut Self>, _: *mut libc::c_void) {
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let (index, _) = server.views.iter().find(|&(_, o)| {
            o.as_ref().get_ref() as *const _ == ctx as *const _
        }).expect("cant find view in arena");
        server.event_queue.push_back(
            Event::XdgSurfaceMap {
                view: index,
            }
        );
    }
    fn xdg_surface_unmap(self: Pin<&mut Self>, _: *mut libc::c_void) {
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let (index, _) = server.views.iter().find(|&(_, o)| {
            o.as_ref().get_ref() as *const _ == ctx as *const _
        }).expect("cant find view in arena");
        server.event_queue.push_back(
            Event::XdgSurfaceUnmap {
                view: index,
            }
        );
    }
    fn xdg_surface_destroy(self: Pin<&mut Self>, _: *mut libc::c_void) {
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let (index, _) = server.views.iter().find(|&(_, o)| {
            o.as_ref().get_ref() as *const _ == ctx as *const _
        }).expect("cant find view in arena");
        let v = server.views.remove(index).expect("cant find view to remove");
        server.dead_views.push(v);

        server.event_queue.push_back(Event::XdgSurfaceDestroy {
            view: index,
        });
    }
    fn xdg_surface_request_move(self: Pin<&mut Self>, _: *mut libc::c_void) {
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let (index, _) = server.views.iter().find(|&(_, o)| {
            o.as_ref().get_ref() as *const _ == ctx as *const _
        }).expect("cant find view in arena");
        server.event_queue.push_back(
            Event::XdgToplevelRequestMove {
                view: index,
            }
        );
    }
    fn xdg_surface_request_resize(self: Pin<&mut Self>, event: *mut wlr_xdg_toplevel_resize_event) {
        let e = unsafe { &*(event) };
        let ctx = unsafe { self.get_unchecked_mut() };
        let server = unsafe { &mut (*ctx.server) };
        let (index, _) = server.views.iter().find(|&(_, o)| {
            o.as_ref().get_ref() as *const _ == ctx as *const _
        }).expect("cant find view in arena");
        server.event_queue.push_back(
            Event::XdgToplevelRequestResize {
                view: index,
                edges: e.edges,
            }
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    CursorMotion {
        time_ms: u32,
        delta_x: f64,
        delta_y: f64,
    },
    CursorMotionAbsolute {
        time_ms: u32,
        x: f64,
        y: f64,
    },
    CursorButton {
        time_ms: u32,
        state: wlr_button_state,
        button: u32,
    },
    CursorAxis {
        time_ms: u32,
        orientation: wlr_axis_orientation,
        source: wlr_axis_source,
        delta: f64,
        delta_discrete: i32,
    },
    CursorFrame,
    OutputFrame {
        output: Index,
        when: std::time::Instant,
    },
    KeyModifier {
        keyboard: Index,
        modifiers: wlr_keyboard_modifiers,
    },
    KeyEvent {
        keyboard: Index,
        state: wlr_key_state,
        keycode: u32,
    },
    XdgToplevelRequestMove {
        view: Index,
    },
    XdgToplevelRequestResize {
        view: Index,
        edges: u32,
    },
    XdgSurfaceMap {
        view: Index,
    },
    XdgSurfaceUnmap {
        view: Index,
    },
    XdgSurfaceNew {
        view: Index,
    },
    XdgSurfaceDestroy {
        view: Index,
    },
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
