#include "ynwl.h"
#include "view.h"
#include "output.h"
#include "keyboard.h"

Ynwl::Ynwl(Server* server): server(server)
{
}

void Ynwl::process_cursor_move()
{
	/* Move the grabbed view to the new position. */
	grabbed_view->x = server->cursor->x - grab_x;
	grabbed_view->y = server->cursor->y - grab_y;
}
void Ynwl::process_cursor_resize()
{
	/*
	 * Resizing the grabbed view can be a little bit complicated, because we
	 * could be resizing from any corner or edge. This not only resizes the view
	 * on one or two axes, but can also move the view if you resize from the top
	 * or left edges (or top-left corner).
	 *
	 * Note that I took some shortcuts here. In a more fleshed-out compositor,
	 * you'd wait for the client to prepare a buffer at the new size, then
	 * commit any movement that was prepared.
	 */
	View *view = grabbed_view;
	double dx = server->cursor->x - grab_x;
	double dy = server->cursor->y - grab_y;
	double x = view->x;
	double y = view->y;
	int width = grab_width;
	int height = grab_height;
	if (resize_edges & WLR_EDGE_TOP) {
		y = grab_y + dy;
		height -= dy;
		if (height < 1) {
			y += height;
		}
	} else if (resize_edges & WLR_EDGE_BOTTOM) {
		height += dy;
	}
	if (resize_edges & WLR_EDGE_LEFT) {
		x = grab_x + dx;
		width -= dx;
		if (width < 1) {
			x += width;
		}
	} else if (resize_edges & WLR_EDGE_RIGHT) {
		width += dx;
	}
	view->x = x;
	view->y = y;
	wlr_xdg_toplevel_set_size(view->xdg_surface, width, height);
}
void Ynwl::process_cursor_motion(uint32_t time)
{
	/* If the mode is non-passthrough, delegate to those functions. */
	if (cursor_mode == CursorMode::Move) {
		process_cursor_move();
		return;
	} else if (cursor_mode == CursorMode::Resize) {
		process_cursor_resize();
		return;
	}

	/* Otherwise, find the view under the pointer and send the event along. */
	double sx, sy;
	struct wlr_surface *surface = NULL;
	View *view = server->get_view_at(server->cursor->x, server->cursor->y, &surface, &sx, &sy);
	if (!view) {
		/* If there's no view under the cursor, set the cursor image to a
		 * default. This is what makes the cursor image appear when you move it
		 * around the screen, not over any views. */
		wlr_xcursor_manager_set_cursor_image(
				server->cursor_mgr, "left_ptr", server->cursor);
	}
	if (surface) {
		bool focus_changed = server->seat->pointer_state.focused_surface != surface;
		/*
		 * "Enter" the surface if necessary. This lets the client know that the
		 * cursor has entered one of its surfaces.
		 *
		 * Note that this gives the surface "pointer focus", which is distinct
		 * from keyboard focus. You get pointer focus by moving the pointer over
		 * a window.
		 */
		wlr_seat_pointer_notify_enter(server->seat, surface, sx, sy);
		if (!focus_changed) {
			/* The enter event contains coordinates, so we only need to notify
			 * on motion if the focus did not change. */
			wlr_seat_pointer_notify_motion(server->seat, time, sx, sy);
		}
	} else {
		/* Clear pointer focus so future button events and such are not sent to
		 * the last client to have the cursor over it. */
		wlr_seat_pointer_clear_focus(server->seat);
	}
}

/* Used to move all of the data necessary to render a surface from the top-level
 * frame handler to the per-surface render function. */
struct render_data {
	struct wlr_output *output;
	struct wlr_renderer *renderer;
	View *view;
	timespec* when;
};

static void render_surface(wlr_surface *surface, int sx, int sy, void *data)
{
	/* This function is called for every surface that needs to be rendered. */
	struct render_data *rdata = (render_data*)data;
	View *view = rdata->view;
	struct wlr_output *output = rdata->output;

	/* We first obtain a wlr_texture, which is a GPU resource. wlroots
	 * automatically handles negotiating these with the client. The underlying
	 * resource could be an opaque handle passed from the client, or the client
	 * could have sent a pixel buffer which we copied to the GPU, or a few other
	 * means. You don't have to worry about this, wlroots takes care of it. */
	struct wlr_texture *texture = wlr_surface_get_texture(surface);
	if (texture == NULL) {
		return;
	}

	/* The view has a position in layout coordinates. If you have two displays,
	 * one next to the other, both 1080p, a view on the rightmost display might
	 * have layout coordinates of 2000,100. We need to translate that to
	 * output-local coordinates, or (2000 - 1920). */
	double ox = 0, oy = 0;
	wlr_output_layout_output_coords(
			view->server->output_layout, output, &ox, &oy);
	ox += view->x + sx, oy += view->y + sy;

	/* We also have to apply the scale factor for HiDPI outputs. This is only
	 * part of the puzzle, TinyWL does not fully support HiDPI. */
	struct wlr_box box = {
		.x = int(ox * output->scale),
		.y = int(oy * output->scale),
		.width = int(surface->current.width * output->scale),
		.height = int(surface->current.height * output->scale),
	};

	/*
	 * Those familiar with OpenGL are also familiar with the role of matricies
	 * in graphics programming. We need to prepare a matrix to render the view
	 * with. wlr_matrix_project_box is a helper which takes a box with a desired
	 * x, y coordinates, width and height, and an output geometry, then
	 * prepares an orthographic projection and multiplies the necessary
	 * transforms to produce a model-view-projection matrix.
	 *
	 * Naturally you can do this any way you like, for example to make a 3D
	 * compositor.
	 */
	float matrix[9];
	enum wl_output_transform transform =
		wlr_output_transform_invert(surface->current.transform);
	wlr_matrix_project_box(matrix, &box, transform, 0,
		output->transform_matrix);

	/* This takes our matrix, the texture, and an alpha, and performs the actual
	 * rendering on the GPU. */
	wlr_render_texture_with_matrix(rdata->renderer, texture, matrix, 1);

	/* This lets the client know that we've displayed that frame and it can
	 * prepare another one now if it likes. */
	wlr_surface_send_frame_done(surface, rdata->when);
}

void Ynwl::output_frame(Output* output,  timespec* when)
{
	struct wlr_renderer *renderer = server->renderer;

	/* wlr_output_attach_render makes the OpenGL context current. */
	if (!wlr_output_attach_render(output->output, NULL)) {
		return;
	}
	/* The "effective" resolution can change if you rotate your outputs. */
	int width, height;
	wlr_output_effective_resolution(output->output, &width, &height);
	/* Begin the renderer (calls glViewport and some other GL sanity checks) */
	wlr_renderer_begin(renderer, width, height);

	float color[4] = {0.3, 0.3, 0.3, 1.0};
	wlr_renderer_clear(renderer, color);

	/* Each subsequent window we render is rendered on top of the last. Because
	 * our view list is ordered front-to-back, we iterate over it backwards. */
	View *view;
	wl_list_for_each_reverse(view, &output->server->views, link) {
		if (!view->mapped) {
			/* An unmapped view should not be rendered. */
			continue;
		}
		struct render_data rdata = {
			.output = output->output,
			.renderer = renderer,
			.view = view,
			.when = when,
		};
		/* This calls our render_surface function for each surface among the
		 * xdg_surface's toplevel and popups. */
		wlr_xdg_surface_for_each_surface(view->xdg_surface,
				render_surface, &rdata);
	}

	/* Hardware cursors are rendered by the GPU on a separate plane, and can be
	 * moved around without re-rendering what's beneath them - which is more
	 * efficient. However, not all hardware supports hardware cursors. For this
	 * reason, wlroots provides a software fallback, which we ask it to render
	 * here. wlr_cursor handles configuring hardware vs software cursors for you,
	 * and this function is a no-op when hardware cursors are in use. */
	wlr_output_render_software_cursors(output->output, NULL);

	/* Conclude rendering and swap the buffers, showing the final frame
	 * on-screen. */
	wlr_renderer_end(renderer);
	wlr_output_commit(output->output);
}

void Ynwl::focus_view(View* view, wlr_surface *surface)
{
	/* Note: this function only deals with keyboard focus. */
	struct wlr_seat *seat = server->seat;
	struct wlr_surface *prev_surface = seat->keyboard_state.focused_surface;
	if (prev_surface == surface) {
		/* Don't re-focus an already focused surface. */
		return;
	}
	if (prev_surface) {
		/*
		 * Deactivate the previously focused surface. This lets the client know
		 * it no longer has focus and the client will repaint accordingly, e.g.
		 * stop displaying a caret.
		 */
		struct wlr_xdg_surface *previous = wlr_xdg_surface_from_wlr_surface(
					seat->keyboard_state.focused_surface);
		wlr_xdg_toplevel_set_activated(previous, false);
	}
	struct wlr_keyboard *keyboard = wlr_seat_get_keyboard(seat);
	/* Move the view to the front */
	wl_list_remove(&view->link);
	wl_list_insert(&server->views, &view->link);
	/* Activate the new surface */
	wlr_xdg_toplevel_set_activated(view->xdg_surface, true);
	/*
	 * Tell the seat to have the keyboard enter this surface. wlroots will keep
	 * track of this and automatically send key events to the appropriate
	 * clients without additional work on your part.
	 */
	wlr_seat_keyboard_notify_enter(seat, view->xdg_surface->surface,
		keyboard->keycodes, keyboard->num_keycodes, &keyboard->modifiers);
}
void Ynwl::begin_interactive(View* view, CursorMode mode, uint32_t edges)
{
	/* This function sets up an interactive move or resize operation, where the
	 * compositor stops propegating pointer events to clients and instead
	 * consumes them itself, to move or resize windows. */
	struct wlr_surface *focused_surface =
		server->seat->pointer_state.focused_surface;
	if (view->xdg_surface->surface != focused_surface) {
		/* Deny move/resize requests from unfocused clients. */
		return;
	}
	grabbed_view = view;
	cursor_mode = mode;
	struct wlr_box geo_box;
	wlr_xdg_surface_get_geometry(view->xdg_surface, &geo_box);
	if (mode == CursorMode::Move) {
		grab_x = server->cursor->x - view->x;
		grab_y = server->cursor->y - view->y;
	} else {
		grab_x = server->cursor->x + geo_box.x;
		grab_y = server->cursor->y + geo_box.y;
	}
	grab_width = geo_box.width;
	grab_height = geo_box.height;
	resize_edges = edges;
}

bool Ynwl::handle_keybinding(xkb_keysym_t sym) {
	/*
	 * Here we handle compositor keybindings. This is when the compositor is
	 * processing keys, rather than passing them on to the client for its own
	 * processing.
	 *
	 * This function assumes Alt is held down.
	 */
	switch (sym) {
		case XKB_KEY_Escape:
			wl_display_terminate(server->display);
			running = false;
			break;
		case XKB_KEY_F1:
		{
			/* Cycle to the next view */
			if (wl_list_length(&server->views) < 2) {
				break;
			}
			View *current_view = wl_container_of(
				server->views.next, current_view, link);
			View *next_view = wl_container_of(
				current_view->link.next, next_view, link);
			focus_view(next_view, next_view->xdg_surface->surface);
			/* Move the previous view to the end of the list */
			wl_list_remove(&current_view->link);
			wl_list_insert(server->views.prev, &current_view->link);
			break;
		}
		default:
			return false;
	}
	return true;
}


void Ynwl::main_loop()
{

	while(running)
	{
		Event e = server->pop_event();
		switch (e.type)
		{
			case EventType::CursorMotion:
			{
				/* The cursor doesn't move unless we tell it to. The cursor automatically
				 * handles constraining the motion to the output layout, as well as any
				 * special configuration applied for the specific input device which
				 * generated the event. You can pass NULL for the device if you want to move
				 * the cursor around without any input. */
				wlr_cursor_move(server->cursor, nullptr,
						e.cursor_motion.delta_x, e.cursor_motion.delta_y);
				process_cursor_motion(e.time_msec);
				break;
			}
			case EventType::CursorMotionAbsolute:
			{
				wlr_cursor_warp_absolute(server->cursor, nullptr, e.cursor_motion_absolute.x, e.cursor_motion_absolute.y);
				process_cursor_motion(e.time_msec);
				break;
			}
			case EventType::CursorButton:
			{

				/* Notify the client with pointer focus that a button press has occurred */
				wlr_seat_pointer_notify_button(server->seat,
						e.time_msec, e.cursor_button.button, e.cursor_button.state);
				double sx, sy;
				struct wlr_surface *surface;
				View *view = server->get_view_at(server->cursor->x, server->cursor->y, &surface, &sx, &sy);
				if (!view)
					break;
				if (e.cursor_button.state == WLR_BUTTON_RELEASED) {
					/* If you released any buttons, we exit interactive move/resize mode. */
					cursor_mode = CursorMode::Passthrough;
				} else {
					/* Focus that client if the button was _pressed_ */
					focus_view(view, surface);
				}
				break;
			}
			case EventType::CursorAxis:
			{
				/* Notify the client with pointer focus of the axis event. */
				wlr_seat_pointer_notify_axis(server->seat,
						e.time_msec, e.cursor_axis.orientation, e.cursor_axis.delta,
						e.cursor_axis.delta_discrete, e.cursor_axis.source);
				break;
			}
			case EventType::CursorFrame:
			{
				/* Notify the client with pointer focus of the frame event. */
				wlr_seat_pointer_notify_frame(server->seat);
				break;
			}
			case EventType::OutputFrame:
			{
				output_frame(e.output_frame.output, &e.output_frame.when);
				break;
			}
			case EventType::KeyModifier:
			{
				/*
				 * A seat can only have one keyboard, but this is a limitation of the
				 * Wayland protocol - not wlroots. We assign all connected keyboards to the
				 * same seat. You can swap out the underlying wlr_keyboard like this and
				 * wlr_seat handles this transparently.
				 */
				wlr_seat_set_keyboard(server->seat, e.key_modifier.keyboard->device);
				/* Send modifiers to the client. */
				wlr_seat_keyboard_notify_modifiers(server->seat,
					&e.key_modifier.modifiers);
				break;
			}
			case EventType::Key:
			{
				/* Translate libinput keycode -> xkbcommon */
				uint32_t keycode = e.key.keycode + 8;
				/* Get a list of keysyms based on the keymap for this keyboard */
				const xkb_keysym_t *syms;
				int nsyms = xkb_state_key_get_syms(
						e.key.keyboard->device->keyboard->xkb_state, keycode, &syms);

				bool handled = false;
				uint32_t modifiers = wlr_keyboard_get_modifiers(e.key.keyboard->device->keyboard);
				if ((modifiers & WLR_MODIFIER_ALT) && e.key.state == WLR_KEY_PRESSED) {
					/* If alt is held down and this button was _pressed_, we attempt to
					 * process it as a compositor keybinding. */
					for (int i = 0; i < nsyms; i++) {
						handled = handle_keybinding(syms[i]);
					}
				}

				if (!handled) {
					/* Otherwise, we pass it along to the client. */
					wlr_seat_set_keyboard(server->seat, e.key.keyboard->device);
					wlr_seat_keyboard_notify_key(server->seat, e.time_msec,
						e.key.keycode, e.key.state);
				}
				break;
			}
			case EventType::XdgToplevelRequestMove:
			{
				begin_interactive(e.xdg_toplevel_request_move.view, CursorMode::Move, 0);
				break;
			}
			case EventType::XdgToplevelRequestResize:
			{
				begin_interactive(e.xdg_toplevel_request_resize.view, CursorMode::Resize, e.xdg_toplevel_request_resize.edges);
				break;
			}
			case EventType::XdgSurfaceMap:
			{
				e.xdg_surface_map.view->mapped = true;
				focus_view(e.xdg_surface_map.view, e.xdg_surface_map.view->xdg_surface->surface);
				break;
			}
			case EventType::XdgSurfaceUnmap:
			{
				e.xdg_surface_unmap.view->mapped = false;
				break;
			}
		}
	}
}
