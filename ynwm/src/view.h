#ifndef __VIEW_H__
#define __VIEW_H__

#include "wlroots.h"
#include "server.h"

struct View {
	wl_list link;
	Server *server;
	wlr_xdg_surface *xdg_surface;
	wl_listener map;
	wl_listener unmap;
	wl_listener destroy;
	wl_listener request_move;
	wl_listener request_resize;
	bool mapped;
	int x, y;

	bool is_at(double lx, double ly, struct wlr_surface **surface,
		double *sx, double *sy);
};

#endif // __VIEW_H__
