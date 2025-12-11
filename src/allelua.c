#include <errno.h>
#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>
#include <stdlib.h>

#define EVENTLOOP_IMPLEMENTATION
#include "src/allelua.h"
#include "src/embed/holy_string.h"

static void al_open_holylibs(lua_State *L)
{
	luaL_dostring(L, (char *)holy_string);
}

int allelua_new(int argc, char **argv, struct allelua **out)
{
	int err;
	struct allelua *al;
	struct evloop_config cfg = {0};

	if (out == NULL)
		return -EINVAL;

	al = calloc(1, sizeof *al);
	if (al == NULL)
		return -ENOMEM;

	err = evloop_init(&al->loop, cfg);
	if (err)
		goto evloop_init_err;

	al->L = luaL_newstate();
	if (al->L == NULL) {
		err = -ENOMEM;
		goto lua_newstate_err;
	}

	luaL_openlibs(al->L);
	al_open_holylibs(al->L);

	// _al global table.
	lua_newtable(al->L);
	// _al.args table.
	lua_newtable(al->L);
	for (int i = 0; i < argc; i++) {
		// Key.
		lua_pushinteger(al->L, i);
		// Value.
		lua_pushstring(al->L, argv[i]);
		// Set t[k] = v.
		lua_settable(al->L, -3);
	}
	lua_setfield(al->L, -2, "args");
	lua_setglobal(al->L, "_al");

	*out = al;
	return 0;

lua_newstate_err:
evloop_init_err:
	free(al);

	return err;
}

void allelua_free(struct allelua *al)
{
	if (al) {
		lua_close(al->L);
		evloop_deinit(&al->loop);
		free(al);
	}
}
