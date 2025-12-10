#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>
#include <stdlib.h>

#include "cmd/allelua/main.h"

int run(int argc, char **argv)
{
	int err = EXIT_SUCCESS;
	lua_State *L = luaL_newstate();
	luaL_openlibs(L);

	(void)argc;
	(void)argv;

	const char *lua_script = "print('Hello world!')";

	if (luaL_dostring(L, lua_script)) {
		fprintf(stderr, "Error: %s\n", lua_tostring(L, -1));
		lua_pop(L, 1);
		err = EXIT_FAILURE;
		goto end;
	}

end:
	lua_close(L);
	return err;
}
