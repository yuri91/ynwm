project(
	'ynwm',
	'c', 'cpp',
	version : '0.0.6',
	default_options : ['warning_level=3', 'c_std=c11', 'cpp_std=c++17']
)

add_project_arguments([
	'-Wall',
	'-Wextra',

	'-DWLR_USE_UNSTABLE',
], language: 'cpp')

wayland_server = dependency('wayland-server')
wlroots = dependency(
	'wlroots',
	version: '0.8.1'
)
xkbcommon = dependency('xkbcommon')

subdir('protocols')

sources = ['src/main.cpp', 'src/server.cpp', 'src/view.cpp', 'src/ynwl.cpp']

executable(
	'ynwm', sources,
	dependencies: [
		wayland_server,
		wayland_protocols,
		wlroots,
		xkbcommon
	],
	include_directories: [
		include_directories('include')
	]
)
