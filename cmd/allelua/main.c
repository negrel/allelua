#include <getopt.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bits/getopt_core.h"
#include "cmd/allelua/main.h"

/**
 * Print root usage menu.
 */
void usage(FILE *out);

/**
 * Print CLI error and exit.
 */
void cli_error(FILE *out, char *fmt, ...);

int main(int argc, char **argv)
{
	int option;

	if (argc < 2)
		cli_error(stderr, "no command provided");

	char *cmd = argv[1];
	if (strcmp(cmd, "run") == 0) {
		return run(argc - 1, argv + 1);
	} else if (strcmp(cmd, "help") == 0 || strcmp(cmd, "-h") == 0 ||
		   strcmp(cmd, "--help") == 0) {
		usage(stderr);
		return EXIT_SUCCESS;
	} else if (argc > 2) {
		while ((option = getopt(argc, argv, "h")) != -1) {
			switch (option) {
			case 'h':
				usage(stderr);
				return EXIT_SUCCESS;
			default:
				return EXIT_FAILURE;
			}
		}
	}

	cli_error(stderr, "unknown command '%s'", cmd);
	return EXIT_SUCCESS;
}

void usage(FILE *out)
{
	fprintf(out, "allelua - a Lua runtime blessed by programming gods.\n");
	fprintf(out, "Alexandre Negrel <alexandre@negrel.dev>\n");
	fprintf(out, "\n");
	fprintf(out, "USAGE:\n");
	fprintf(out, "    allelua COMMAND [ARGS...]\n");
	fprintf(out, "    allelua run ./main.lua\n");
	fprintf(out, "\n");
	fprintf(out, "COMMANDS:\n");
	fprintf(out, "    run                  Execute a Lua file.\n");
	fprintf(out, "    help COMMAND         Print help menu of COMMAND.\n");
	fprintf(out, "\n");
	fprintf(out, "FLAGS:\n");
	fprintf(out, "    -h, --help           Print help menu.\n");
	fprintf(out, "\n");
	fprintf(out, "Source code: https://github.com/negrel/allelua\n");
	fflush(out);
}

void cli_error(FILE *out, char *fmt, ...)
{
	va_list args;
	va_start(args, fmt);

	fprintf(out, "Error: ");
	vfprintf(out, fmt, args);
	fprintf(out, "\n\n");
	fprintf(out, "USAGE: allelua COMMAND [ARGS...]\n");
	fprintf(out, "\n");
	fprintf(out, "Run 'allelua -h' for more informations\n");
	fflush(out);

	va_end(args);

	exit(EXIT_FAILURE);
}
