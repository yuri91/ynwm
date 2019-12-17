#ifndef __KEYBOARD_H__
#define __KEYBOARD_H__

#include "wlroots.h"
#include "server.h"

struct Keyboard {
	wl_list link;
	Server *server;
	wlr_input_device *device;

	wl_listener modifiers;
	wl_listener key;
};

#endif // __KEYBOARD_H__
