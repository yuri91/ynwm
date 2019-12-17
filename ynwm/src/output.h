#ifndef __OUTPUT_H__
#define __OUTPUT_H__

#include "server.h"

struct Output {
	wl_list link;
	Server *server;
	wlr_output *output;
	wl_listener frame;
};

#endif // __OUTPUT_H__
