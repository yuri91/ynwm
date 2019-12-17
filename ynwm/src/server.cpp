#include "server.h"
#include "view.h"
#include "keyboard.h"
#include "output.h"

static void server_new_input(struct wl_listener *listener, void *data) {
	/* This event is raised by the backend when a new input device becomes
	 * available. */
	Server *server =
		wl_container_of(listener, server, new_input);
	struct wlr_input_device *device = (wlr_input_device*)data;
	switch (device->type) {
		case WLR_INPUT_DEVICE_KEYBOARD:
			server->new_keyboard(device);
			break;
		case WLR_INPUT_DEVICE_POINTER:
			server->new_pointer(device);
			break;
		default:
			break;
	}
	/* We need to let the wlr_seat know what our capabilities are, which is
	 * communiciated to the client. In TinyWL we always have a cursor, even if
	 * there are no pointer devices, so we always include that capability. */
	uint32_t caps = WL_SEAT_CAPABILITY_POINTER;
	if (!wl_list_empty(&server->keyboards)) {
		caps |= WL_SEAT_CAPABILITY_KEYBOARD;
	}
	wlr_seat_set_capabilities(server->seat, caps);
}

static void seat_request_cursor(struct wl_listener *listener, void *data) {
	Server *server = wl_container_of(
			listener, server, request_cursor);
	/* This event is rasied by the seat when a client provides a cursor image */
	struct wlr_seat_pointer_request_set_cursor_event *event =(wlr_seat_pointer_request_set_cursor_event*) data;
	struct wlr_seat_client *focused_client =
		server->seat->pointer_state.focused_client;
	/* This can be sent by any client, so we check to make sure this one is
	 * actually has pointer focus first. */
	if (focused_client == event->seat_client) {
		/* Once we've vetted the client, we can tell the cursor to use the
		 * provided surface as the cursor image. It will set the hardware cursor
	* on the output that it's currently on and continue to do so as the
		 * cursor moves between outputs. */
		wlr_cursor_set_surface(server->cursor, event->surface,
				event->hotspot_x, event->hotspot_y);
	}
}

static void server_cursor_motion(struct wl_listener *listener, void *data) {
	/* This event is forwarded by the cursor when a pointer emits a _relative_
	 * pointer motion event (i.e. a delta) */
	Server *server =
		wl_container_of(listener, server, cursor_motion);
	struct wlr_event_pointer_motion *event = (wlr_event_pointer_motion*)data;
	server->event_queue.push(Event::new_cursor_motion(event->time_msec, event->delta_x, event->delta_y));
}

static void server_cursor_motion_absolute(
		struct wl_listener *listener, void *data) {
	/* This event is forwarded by the cursor when a pointer emits an _absolute_
	 * motion event, from 0..1 on each axis. This happens, for example, when
	 * wlroots is running under a Wayland window rather than KMS+DRM, and you
	 * move the mouse over the window. You could enter the window from any edge,
	 * so we have to warp the mouse there. There is also some hardware which
	 * emits these events. */
	Server *server =
		wl_container_of(listener, server, cursor_motion_absolute);
	struct wlr_event_pointer_motion_absolute *event = (wlr_event_pointer_motion_absolute*)data;
	server->event_queue.push(Event::new_cursor_motion_absolute(event->time_msec, event->x, event->y));
}

static void server_cursor_button(struct wl_listener *listener, void *data) {
	/* This event is forwarded by the cursor when a pointer emits a button
	 * event. */
	Server *server =
		wl_container_of(listener, server, cursor_button);
	struct wlr_event_pointer_button *event = (wlr_event_pointer_button*)data;
	server->event_queue.push(Event::new_cursor_button(event->time_msec, event->state, event->button));
}

static void server_cursor_axis(struct wl_listener *listener, void *data) {
	/* This event is forwarded by the cursor when a pointer emits an axis event,
	 * for example when you move the scroll wheel. */
	Server *server =
		wl_container_of(listener, server, cursor_axis);
	struct wlr_event_pointer_axis *event = (wlr_event_pointer_axis*)data;
	server->event_queue.push(Event::new_cursor_axis(event->time_msec, event->orientation, event->source, event->delta, event->delta_discrete));
}

static void server_cursor_frame(struct wl_listener *listener, void *data) {
	(void)data;
	/* This event is forwarded by the cursor when a pointer emits an frame
	 * event. Frame events are sent after regular pointer events to group
	 * multiple events together. For instance, two axis events may happen at the
	 * same time, in which case a frame event won't be sent in between. */
	Server *server =
		wl_container_of(listener, server, cursor_frame);
	server->event_queue.push(Event::new_cursor_frame());
}

static void output_frame(struct wl_listener *listener, void *data) {
	(void)data;
	/* This function is called every time an output is ready to display a frame,
	 * generally at the output's refresh rate (e.g. 60Hz). */
	Output *output =
		wl_container_of(listener, output, frame);

	struct timespec now;
	clock_gettime(CLOCK_MONOTONIC, &now);

	output->server->event_queue.push(Event::new_output_frame(now, output));
}

static void server_new_output(struct wl_listener *listener, void *data) {
	/* This event is rasied by the backend when a new output (aka a display or
	 * monitor) becomes available. */
	Server *server =
		wl_container_of(listener, server, new_output);
	struct wlr_output *wlr_output_ = (wlr_output*)data;

	/* Some backends don't have modes. DRM+KMS does, and we need to set a mode
	 * before we can use the output. The mode is a tuple of (width, height,
	 * refresh rate), and each monitor supports only a specific set of modes. We
	 * just pick the first, a more sophisticated compositor would let the user
	 * configure it or pick the mode the display advertises as preferred. */
	if (!wl_list_empty(&wlr_output_->modes)) {
		struct wlr_output_mode *mode =
			wl_container_of(wlr_output_->modes.prev, mode, link);
		wlr_output_set_mode(wlr_output_, mode);
	}

	/* Allocates and configures our state for this output */
	Output *output = new Output;
	output->output = wlr_output_;
	output->server = server;
	/* Sets up a listener for the frame notify event. */
	output->frame.notify = output_frame;
	wl_signal_add(&wlr_output_->events.frame, &output->frame);
	wl_list_insert(&server->outputs, &output->link);

	/* Adds this to the output layout. The add_auto function arranges outputs
	 * from left-to-right in the order they appear. A more sophisticated
	 * compositor would let the user configure the arrangement of outputs in the
	 * layout. */
	wlr_output_layout_add_auto(server->output_layout, wlr_output_);

	/* Creating the global adds a wl_output global to the display, which Wayland
	 * clients can see to find out information about the output (such as
	 * DPI, scale factor, manufacturer, etc). */
	wlr_output_create_global(wlr_output_);
}

static void xdg_surface_map(struct wl_listener *listener, void *data) {
	(void)data;
	/* Called when the surface is mapped, or ready to display on-screen. */
	View *view = wl_container_of(listener, view, map);
	view->server->push_event(Event::new_xdg_surface_map(view));
}

static void xdg_surface_unmap(struct wl_listener *listener, void *data) {
	(void)data;
	/* Called when the surface is unmapped, and should no longer be shown. */
	View *view = wl_container_of(listener, view, unmap);
	view->server->push_event(Event::new_xdg_surface_unmap(view));
}

static void xdg_surface_destroy(struct wl_listener *listener, void *data) {
	(void)data;
	/* Called when the surface is destroyed and should never be shown again. */
	View *view = wl_container_of(listener, view, destroy);
	wl_list_remove(&view->link);
	//TODO: What if view was the grabbed_view?
	delete view;
}

static void xdg_toplevel_request_move(
		struct wl_listener *listener, void *data) {
	(void)data;
	/* This event is raised when a client would like to begin an interactive
	 * move, typically because the user clicked on their client-side
	 * decorations. Note that a more sophisticated compositor should check the
	 * provied serial against a list of button press serials sent to this
	 * client, to prevent the client from requesting this whenever they want. */
	View *view = wl_container_of(listener, view, request_move);
	view->server->push_event(Event::new_xdg_toplevel_request_move(view));
}

static void xdg_toplevel_request_resize(
		struct wl_listener *listener, void *data) {
	/* This event is raised when a client would like to begin an interactive
	 * resize, typically because the user clicked on their client-side
	 * decorations. Note that a more sophisticated compositor should check the
	 * provied serial against a list of button press serials sent to this
	 * client, to prevent the client from requesting this whenever they want. */
	struct wlr_xdg_toplevel_resize_event *event = (wlr_xdg_toplevel_resize_event*)data;
	View *view = wl_container_of(listener, view, request_resize);
	view->server->push_event(Event::new_xdg_toplevel_request_resize(view, event->edges));
}

static void server_new_xdg_surface(struct wl_listener *listener, void *data) {
	/* This event is raised when wlr_xdg_shell receives a new xdg surface from a
	 * client, either a toplevel (application window) or popup. */
	Server *server =
		wl_container_of(listener, server, new_xdg_surface);
	struct wlr_xdg_surface *xdg_surface = (wlr_xdg_surface*)data;
	if (xdg_surface->role != WLR_XDG_SURFACE_ROLE_TOPLEVEL) {
		return;
	}

	/* Allocate a View for this surface */
	View *view = new View;
	view->server = server;
	view->xdg_surface = xdg_surface;

	/* Listen to the various events it can emit */
	view->map.notify = xdg_surface_map;
	wl_signal_add(&xdg_surface->events.map, &view->map);
	view->unmap.notify = xdg_surface_unmap;
	wl_signal_add(&xdg_surface->events.unmap, &view->unmap);
	view->destroy.notify = xdg_surface_destroy;
	wl_signal_add(&xdg_surface->events.destroy, &view->destroy);

	/* cotd */
	struct wlr_xdg_toplevel *toplevel = xdg_surface->toplevel;
	view->request_move.notify = xdg_toplevel_request_move;
	wl_signal_add(&toplevel->events.request_move, &view->request_move);
	view->request_resize.notify = xdg_toplevel_request_resize;
	wl_signal_add(&toplevel->events.request_resize, &view->request_resize);

	/* Add it to the list of views. */
	wl_list_insert(&server->views, &view->link);
}

Server::Server()
{
	/* The Wayland display is managed by libwayland. It handles accepting
	 * clients from the Unix socket, manging Wayland globals, and so on. */
	display = wl_display_create();
	/* The backend is a wlroots feature which abstracts the underlying input and
	 * output hardware. The autocreate option will choose the most suitable
	 * backend based on the current environment, such as opening an X11 window
	 * if an X11 server is running. The NULL argument here optionally allows you
	 * to pass in a custom renderer if wlr_renderer doesn't meet your needs. The
	 * backend uses the renderer, for example, to fall back to software cursors
	 * if the backend does not support hardware cursors (some older GPUs
	 * don't). */
	backend = wlr_backend_autocreate(display, NULL);

	/* If we don't provide a renderer, autocreate makes a GLES2 renderer for us.
	 * The renderer is responsible for defining the various pixel formats it
	 * supports for shared memory, this configures that for clients. */
	renderer = wlr_backend_get_renderer(backend);
	wlr_renderer_init_wl_display(renderer, display);

	/* This creates some hands-off wlroots interfaces. The compositor is
	 * necessary for clients to allocate surfaces and the data device manager
	 * handles the clipboard. Each of these wlroots interfaces has room for you
	 * to dig your fingers in and play with their behavior if you want. */
	wlr_compositor_create(display, renderer);
	wlr_data_device_manager_create(display);

	/* Creates an output layout, which a wlroots utility for working with an
	 * arrangement of screens in a physical layout. */
	output_layout = wlr_output_layout_create();

	/* Configure a listener to be notified when new outputs are available on the
	 * backend. */
	wl_list_init(&outputs);
	new_output.notify = server_new_output;
	wl_signal_add(&backend->events.new_output, &new_output);

	/* Set up our list of views and the xdg-shell. The xdg-shell is a Wayland
	 * protocol which is used for application windows. For more detail on
	 * shells, refer to my article:
	 *
	 * https://drewdevault.com/2018/07/29/Wayland-shells.html
	 */
	wl_list_init(&views);
	xdg_shell = wlr_xdg_shell_create(display);
	new_xdg_surface.notify = server_new_xdg_surface;
	wl_signal_add(&xdg_shell->events.new_surface, &new_xdg_surface);

	/*
	 * Creates a cursor, which is a wlroots utility for tracking the cursor
	 * image shown on screen.
	 */
	cursor = wlr_cursor_create();
	wlr_cursor_attach_output_layout(cursor, output_layout);

	/* Creates an xcursor manager, another wlroots utility which loads up
	 * Xcursor themes to source cursor images from and makes sure that cursor
	 * images are available at all scale factors on the screen (necessary for
	 * HiDPI support). We add a cursor theme at scale factor 1 to begin with. */
	cursor_mgr = wlr_xcursor_manager_create(NULL, 24);
	wlr_xcursor_manager_load(cursor_mgr, 1);

	/*
	 * wlr_cursor *only* displays an image on screen. It does not move around
	 * when the pointer moves. However, we can attach input devices to it, and
	 * it will generate aggregate events for all of them. In these events, we
	 * can choose how we want to process them, forwarding them to clients and
	 * moving the cursor around. More detail on this process is described in my
	 * input handling blog post:
	 *
	 * https://drewdevault.com/2018/07/17/Input-handling-in-wlroots.html
	 *
	 * And more comments are sprinkled throughout the notify functions above.
	 */
	cursor_motion.notify = server_cursor_motion;
	wl_signal_add(&cursor->events.motion, &cursor_motion);
	cursor_motion_absolute.notify = server_cursor_motion_absolute;
	wl_signal_add(&cursor->events.motion_absolute,
			&cursor_motion_absolute);
	cursor_button.notify = server_cursor_button;
	wl_signal_add(&cursor->events.button, &cursor_button);
	cursor_axis.notify = server_cursor_axis;
	wl_signal_add(&cursor->events.axis, &cursor_axis);
	cursor_frame.notify = server_cursor_frame;
	wl_signal_add(&cursor->events.frame, &cursor_frame);

	/*
	 * Configures a seat, which is a single "seat" at which a user sits and
	 * operates the computer. This conceptually includes up to one keyboard,
	 * pointer, touch, and drawing tablet device. We also rig up a listener to
	 * let us know when new input devices are available on the backend.
	 */
	wl_list_init(&keyboards);
	new_input.notify = server_new_input;
	wl_signal_add(&backend->events.new_input, &new_input);
	seat = wlr_seat_create(display, "seat0");
	request_cursor.notify = seat_request_cursor;
	wl_signal_add(&seat->events.request_set_cursor,
			&request_cursor);

	/* Add a Unix socket to the Wayland display. */
	const char *socket = wl_display_add_socket_auto(display);
	if (!socket) {
		wlr_backend_destroy(backend);
		exit(1);
	}

	/* Start the backend. This will enumerate outputs and inputs, become the DRM
	 * master, etc */
	if (!wlr_backend_start(backend)) {
		wlr_backend_destroy(backend);
		wl_display_destroy(display);
		exit(1);
	}

	/* Set the WAYLAND_DISPLAY environment variable to our socket and run the
	 * startup command if requested. */
	setenv("WAYLAND_DISPLAY", socket, true);
}
Server::~Server()
{

	wl_display_destroy_clients(display);
	wl_display_destroy(display);
}

void Server::push_event(Event e)
{
	event_queue.push(e);
}

Event Server::pop_event()
{
	while (event_queue.empty())
	{
		wl_event_loop* loop = wl_display_get_event_loop(display);
		wl_display_flush_clients(display);
		wl_event_loop_dispatch(loop, -1);
	}
	Event e = event_queue.front();
	event_queue.pop();
	return e;
}

static void keyboard_handle_modifiers(
		struct wl_listener *listener, void *data) {
	(void)data;
	/* This event is raised when a modifier key, such as shift or alt, is
	 * pressed. We simply communicate this to the client. */
	Keyboard *keyboard =
		wl_container_of(listener, keyboard, modifiers);
	keyboard->server->push_event(Event::new_key_modifier(keyboard, keyboard->device->keyboard->modifiers));
}


static void keyboard_handle_key(
		struct wl_listener *listener, void *data) {
	/* This event is raised when a key is pressed or released. */
	Keyboard *keyboard =
		wl_container_of(listener, keyboard, key);
	struct wlr_event_keyboard_key *event = (wlr_event_keyboard_key*)data;
	keyboard->server->push_event(Event::new_key(event->time_msec, keyboard, event->state, event->keycode));
}

void Server::new_keyboard(struct wlr_input_device *device) {
	Keyboard *keyboard = new Keyboard;
	keyboard->server = this;
	keyboard->device = device;

	/* We need to prepare an XKB keymap and assign it to the keyboard. This
	 * assumes the defaults (e.g. layout = "us"). */
	struct xkb_rule_names rules = {};
	struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
	struct xkb_keymap *keymap = xkb_map_new_from_names(context, &rules,
		XKB_KEYMAP_COMPILE_NO_FLAGS);

	wlr_keyboard_set_keymap(device->keyboard, keymap);
	xkb_keymap_unref(keymap);
	xkb_context_unref(context);
	wlr_keyboard_set_repeat_info(device->keyboard, 25, 600);

	/* Here we set up listeners for keyboard events. */
	keyboard->modifiers.notify = keyboard_handle_modifiers;
	wl_signal_add(&device->keyboard->events.modifiers, &keyboard->modifiers);
	keyboard->key.notify = keyboard_handle_key;
	wl_signal_add(&device->keyboard->events.key, &keyboard->key);

	wlr_seat_set_keyboard(seat, device);

	/* And add the keyboard to our list of keyboards */
	wl_list_insert(&keyboards, &keyboard->link);
}

void Server::new_pointer(struct wlr_input_device *device) {
	/* We don't do anything special with pointers. All of our pointer handling
	 * is proxied through wlr_cursor. On another compositor, you might take this
	 * opportunity to do libinput configuration on the device to set
	 * acceleration, etc. */
	wlr_cursor_attach_input_device(cursor, device);
}

View* Server::get_view_at(double lx, double ly,
		struct wlr_surface **surface, double *sx, double *sy) {
	/* This iterates over all of our surfaces and attempts to find one under the
	 * cursor. This relies on views being ordered from top-to-bottom. */
	View *view;
	wl_list_for_each(view, &views, link) {
		if (view->is_at(lx, ly, surface, sx, sy)) {
			return view;
		}
	}
	return NULL;
}


