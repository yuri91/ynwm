#ifndef __EVENT_H__
#define __EVENT_H__

#include "wlroots.h"

struct Output;
struct View;
struct Keyboard;

enum class EventType {
	CursorMotion,
	CursorMotionAbsolute,
	CursorButton,
	CursorAxis,
	CursorFrame,

	OutputFrame,

	KeyModifier,
	Key,

	XdgToplevelRequestMove,
	XdgToplevelRequestResize,

	XdgSurfaceMap,
	XdgSurfaceUnmap,
};
struct CursorMotionEvent {
	double delta_x;
	double delta_y;
};
struct CursorMotionAbsoluteEvent {
	double x;
	double y;
};
struct CursorButtonEvent {
	wlr_button_state state;
	uint32_t button;
};
struct CursorAxisEvent {
	wlr_axis_orientation orientation;
	wlr_axis_source source;
	double delta;
	int32_t delta_discrete;
};
struct CursorFrameEvent {
};
struct OutputFrameEvent {
	Output* output;
	timespec when;
};
struct KeyModifierEvent {
	Keyboard* keyboard;
	wlr_keyboard_modifiers modifiers;
};
struct KeyEvent {
	Keyboard* keyboard;
	wlr_key_state state;
	uint32_t keycode;
};
struct XdgToplevelRequestMoveEvent {
	View* view;
};
struct XdgToplevelRequestResizeEvent {
	View* view;
	uint32_t edges;
};
struct XdgSurfaceMapEvent {
	View* view;
};
struct XdgSurfaceUnmapEvent {
	View* view;
};

struct Event {
	EventType type;
	union {
		CursorMotionEvent cursor_motion;
		CursorMotionAbsoluteEvent cursor_motion_absolute;
		CursorButtonEvent cursor_button;
		CursorAxisEvent cursor_axis;
		CursorFrameEvent cursor_frame;
		OutputFrameEvent output_frame;
		KeyModifierEvent key_modifier;
		KeyEvent key;
		XdgToplevelRequestMoveEvent xdg_toplevel_request_move;
		XdgToplevelRequestResizeEvent xdg_toplevel_request_resize;
		XdgSurfaceMapEvent xdg_surface_map;
		XdgSurfaceUnmapEvent xdg_surface_unmap;
	};
	uint32_t time_msec;

	static Event new_cursor_motion(uint32_t time_msec, double delta_x, double delta_y)
	{
		Event e;
		e.type = EventType::CursorMotion;
		e.time_msec = time_msec;
		e.cursor_motion.delta_x = delta_x;
		e.cursor_motion.delta_y = delta_y;
		return e;
	}
	static Event new_cursor_motion_absolute(uint32_t time_msec, double x, double y)
	{
		Event e;
		e.type = EventType::CursorMotionAbsolute;
		e.time_msec = time_msec;
		e.cursor_motion_absolute.x = x;
		e.cursor_motion_absolute.y = y;
		return e;
	}
	static Event new_cursor_button(uint32_t time_msec, wlr_button_state state, uint32_t button)
	{
		Event e;
		e.type = EventType::CursorButton;
		e.time_msec = time_msec;
		e.cursor_button.state = state;
		e.cursor_button.button = button;
		return e;
	}
	static Event new_cursor_axis(uint32_t time_msec, wlr_axis_orientation o, wlr_axis_source s, double d, int32_t dd)
	{
		Event e;
		e.type = EventType::CursorAxis;
		e.time_msec = time_msec;
		e.cursor_axis.orientation = o;
		e.cursor_axis.source = s;
		e.cursor_axis.delta = d;
		e.cursor_axis.delta_discrete = dd;
		return e;
	}
	static Event new_cursor_frame()
	{
		Event e;
		e.type = EventType::CursorFrame;
		e.time_msec = 0;
		return e;
	}
	static Event new_output_frame(timespec when, Output* output)
	{
		Event e;
		e.type = EventType::OutputFrame;
		e.time_msec = when.tv_sec*1000 + when.tv_nsec/1000000;
		e.output_frame.output = output;
		e.output_frame.when = when;
		return e;
	}
	static Event new_key_modifier(Keyboard* keyboard, wlr_keyboard_modifiers modifiers)
	{
		Event e;
		e.type = EventType::KeyModifier;
		e.time_msec = 0;
		e.key_modifier.keyboard = keyboard;
		e.key_modifier.modifiers = modifiers;
		return e;
	}
	static Event new_key(uint32_t time_msec, Keyboard* keyboard, wlr_key_state state, uint32_t keycode)
	{
		Event e;
		e.type = EventType::KeyModifier;
		e.time_msec = time_msec;
		e.key.keyboard = keyboard;
		e.key.state = state;
		e.key.keycode = keycode;
		return e;
	}
	static Event new_xdg_toplevel_request_move(View* view)
	{
		Event e;
		e.type = EventType::XdgToplevelRequestMove;
		e.time_msec = 0;
		e.xdg_toplevel_request_move.view = view;
		return e;
	}
	static Event new_xdg_toplevel_request_resize(View* view, uint32_t edges)
	{
		Event e;
		e.type = EventType::XdgToplevelRequestResize;
		e.time_msec = 0;
		e.xdg_toplevel_request_resize.view = view;
		e.xdg_toplevel_request_resize.edges = edges;
		return e;
	}
	static Event new_xdg_surface_map(View* view)
	{
		Event e;
		e.type = EventType::XdgSurfaceMap;
		e.time_msec = 0;
		e.xdg_surface_map.view = view;
		return e;
	}
	static Event new_xdg_surface_unmap(View* view)
	{
		Event e;
		e.type = EventType::XdgSurfaceUnmap;
		e.time_msec = 0;
		e.xdg_surface_unmap.view = view;
		return e;
	}
};

#endif // __EVENT_H__
