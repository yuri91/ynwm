#ifndef __SERVER_H__
#define __SERVER_H__

#include <queue>

#include "wlroots.h"

#include "event.h"

struct Output;
struct View;

enum class CursorMode {
	Passthrough,
	Move,
	Resize,
};

struct Server {
	wl_display *display;
	wlr_backend *backend;
	wlr_renderer *renderer;

	wlr_xdg_shell *xdg_shell;
	wl_listener new_xdg_surface;
	wl_list views;

	wlr_cursor *cursor;
	wlr_xcursor_manager *cursor_mgr;
	wl_listener cursor_motion;
	wl_listener cursor_motion_absolute;
	wl_listener cursor_button;
	wl_listener cursor_axis;
	wl_listener cursor_frame;

	wlr_seat *seat;
	wl_listener new_input;
	wl_listener request_cursor;
	wl_list keyboards;

	wlr_output_layout *output_layout;
	wl_list outputs;
	wl_listener new_output;

	std::queue<Event> event_queue;

	void dispatch_events();

	void push_event(Event e);
	Event pop_event();

	void new_keyboard(struct wlr_input_device *device);
	void new_pointer(struct wlr_input_device *device);

	void output_frame(Output* output, timespec* when);

	Server();
	~Server();

	View *get_view_at(double lx, double ly,
			struct wlr_surface **surface, double *sx, double *sy);

};

#endif // __SERVER_H__
