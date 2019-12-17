wayland_protocols = dependency('wayland-protocols')
wayland_proto_dir = wayland_protocols.get_pkgconfig_variable('pkgdatadir')
wayland_scanner = find_program('wayland-scanner')

# according to sway, private-code used to be code pre 1.14.91?
wayland_scanner_code = generator(
	wayland_scanner,
	output: '@BASENAME@-protocol.c',
	arguments: ['private-code', '@INPUT@', '@OUTPUT@']
)

wayland_scanner_header = generator(
	wayland_scanner,
	output: '@BASENAME@-protocol.h',
	arguments: ['server-header', '@INPUT@', '@OUTPUT@']
)

protocols = [
	join_paths(wayland_proto_dir, 'stable/xdg-shell/xdg-shell.xml')
]

proto_sources = []
proto_headers = []

foreach p : protocols
	proto_sources += wayland_scanner_code.process(p)
	proto_headers += wayland_scanner_header.process(p)
endforeach

lib_protocols = static_library(
	'protocols',
	proto_sources + proto_headers,
	dependencies: [wayland_server]
)

wayland_protocols = declare_dependency(
	sources: proto_headers,
	link_with: lib_protocols
)
