#include <getopt.h>
#include <cstdlib>
#include <cstdio>
#include <time.h>
#include <unistd.h>

#include "wlroots.h"
#include "server.h"
#include "ynwl.h"


int main(int argc, char *argv[]) {
	wlr_log_init(WLR_DEBUG, NULL);
	char *startup_cmd = NULL;

	int c;
	while ((c = getopt(argc, argv, "s:h")) != -1) {
		switch (c) {
			case 's':
				startup_cmd = optarg;
				break;
			default:
				printf("Usage: %s [-s startup command]\n", argv[0]);
				return 0;
		}
	}
	if (optind < argc) {
		printf("Usage: %s [-s startup command]\n", argv[0]);
		return 0;
	}

	Server server;
	if (startup_cmd) {
		if (fork() == 0) {
			execl("/bin/sh", "/bin/sh", "-c", startup_cmd, (void *)NULL);
		}
	}

	Ynwl ynwl(&server);
	ynwl.main_loop();

	return 0;
}
