#ifndef __YNWL_H__
#define __YNWL_H__

#include "wlroots.h"
#include "server.h"

class Ynwl
{
	Server* server;
	View* grabbed_view = nullptr;
	CursorMode cursor_mode = CursorMode::Passthrough;
	double grab_x=0, grab_y=0;
	int grab_width=0, grab_height=0;
	uint32_t resize_edges=0;
	bool running = true;

public:
	Ynwl(Server* server);
	void main_loop();
private:
	void process_cursor_move();
	void process_cursor_resize();
	void process_cursor_motion(uint32_t time);
	void output_frame(Output* output,  timespec* when);
	void focus_view(View* view, wlr_surface *surface);
	void begin_interactive(View* view, CursorMode mode, uint32_t edges);
	bool handle_keybinding(xkb_keysym_t sym);
};

#endif// __YNWL_H__

