#ifndef ALLELUA_H_INCLUDE
#define ALLELUA_H_INCLUDE

#include <lua.h>

#include "eventloop.h"

/**
 * The Allelua runtime, made of a Lua VM and an event loop.
 */
struct allelua {
	lua_State *L;
	struct evloop loop;
};

/**
 * Initialize a new runtime. A negative errno is returned on error.
 */
int allelua_new(int argc, char **argv, struct allelua **al);

/**
 * Free the runtime and associated resources.
 */
void allelua_free(struct allelua *al);

#endif /* ALLELUA_H_INCLUDE */
