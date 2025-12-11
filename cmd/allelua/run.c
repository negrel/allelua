#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>
#include <stdlib.h>
#include <string.h>

#include "cmd/allelua/main.h"

/**
 * Print CLI error and exit.
 */
#define CLI_ERRORF(out, fmt, ...)                                              \
	do {                                                                   \
		fprintf(out, "Error: ");                                       \
		fprintf(out, fmt, __VA_ARGS__);                                \
		fprintf(out, "\n\n");                                          \
		fprintf(out, "USAGE: allelua run FILE [ARGS...]\n");           \
		fprintf(out, "\n");                                            \
		fprintf(out, "Run 'allelua run -h' for more informations\n");  \
		fflush(out);                                                   \
		exit(EXIT_FAILURE);                                            \
	} while (0)
#define CLI_ERROR(out, msg) CLI_ERRORF(out, "%s", msg)

int run(int argc, char **argv)
{
	int err = EXIT_SUCCESS;

	if (argc < 2)
		CLI_ERROR(stderr, "no Lua file specified");

	char *lua_file = argv[1];

	if (strcmp(lua_file, "help") == 0 || strcmp(lua_file, "-h") == 0 ||
	    strcmp(lua_file, "--help") == 0) {
		usage(stderr);
		return EXIT_SUCCESS;
	} else if (strcmp(lua_file, "-") == 0) {
		lua_file = NULL;
	} else {
		CLI_ERRORF(stderr, "unknown flag '%s'", argv[1]);
		return EXIT_FAILURE;
	}

	lua_State *L = luaL_newstate();
	luaL_openlibs(L);

	luaL_loadfile(L, lua_file);
	if (lua_pcall(L, 0, 0, 0)) {
		fprintf(stderr, "Error: %s\n", lua_tostring(L, -1));
		lua_pop(L, 1);
		err = EXIT_FAILURE;
		goto end;
	}

end:
	lua_close(L);
	return err;
}

static void usage(FILE *out)
{
	fprintf(out, "allelua run - Run a Lua program.\n");
	fprintf(out, "\n");
	fprintf(out, "USAGE:\n");
	fprintf(out, "    allelua run [FLAGS] FILE [ARGS...]\n");
	fprintf(out, "    allelua run greet.lua john \n");
	fprintf(out, "\n");
	fprintf(out, "FLAGS:\n");
	fprintf(out, "    -                    Read file from stdin\n");
	fprintf(out, "    -h, --help           Print help menu.\n");
	fflush(out);
}
