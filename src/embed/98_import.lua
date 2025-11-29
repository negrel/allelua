local function is_file_modname(modname)
	return string.has_prefix(modname, "./") or string.has_suffix(modname, ".lua")
end

--- Normalize module name into a suitable Lua identifier. For example,
--- './my/lib.lua' become 'my_lib'.
local function mod_identifier(modname)
	if is_file_modname(modname) then
		modname = resolve_path(modname)
		modname = string.strip_prefix(modname, "./")
		repeat
			modname = string.strip_prefix(modname, "../")
		until not string.has_prefix(modname, "../")
		modname = string.replace_all(modname, "/", "_")
	end

	modname = string.strip_suffix(modname, ".lua")

	return modname
end

--- Import a Lua module into current scope.
---
--- ```lua
--- import { foo = "bar" }  -- import module bar as foo
--- import { "bar", "baz" } -- import bar and baz modules
--- import "bar"            -- import bar module
--- ```
local function user_import(module)
	local import_mod = function(modname, importname)
		module[mod_identifier(importname)] = require(modname)

		if is_file_modname(modname) then
			package.loaded[modname] = nil
		end
	end

	return function(modname)
		if type(modname) == "table" then
			for importname, modname in pairs(modname) do
				if type(importname) == "string" then
					import_mod(modname, importname)
				else
					import_mod(modname, modname)
				end
			end
		elseif type(modname) == "string" then
			import_mod(modname, modname)
		else
			error("module name must be a string")
		end
	end
end

--- Allelua custom loader to import Lua file.
local function file_loader(modname)
	modname = real_path(modname)
	if package.loaded[modname] then return package.loaded[modname] end

	local m = Module.file(modname)
	m = m:load()
	package.loaded[modname] = m

	return m
end

--- Allelua custom searcher to import Lua file.
local function file_searcher(modname)
	if not is_file_modname(modname) then return nil end

	return file_loader, modname
end

-- Replace with our custom Lua loader.
package.searchers = {
	package.searchers[1], -- Preload loader.
	file_searcher,
}
package.loaders = package.searchers

-- Remove I/O Lua built-in libs.
for k in pairs(package.loaded) do package.loaded[k] = nil end
package.loaded.math = math
package.loaded.string = string
package.loaded.table = table

-- Module class.
Module = { __metatable = "Module" }
Module.__index = Module

-- Create a new Module object.
function Module.new(func)
	local global = {
		dump = dump,
		error = error,
		ipairs = ipairs,
		nursery = nursery,
		pairs = pairs,
		pcall = pcall,
		raise = raise,
		traceback = debug.traceback,
		type = type,
	}
	local env = setmetatable({}, { __index = global })
	env.module = env

	global.import = user_import(env)

	return setmetatable({
		global = global,
		env = env,
		func = setfenv(func, env),
	}, Module)
end

-- Create a new Module object for given file.
function Module.file(fpath)
	return Module.new(assert(loadfile(fpath), "file not found"))
end

-- Load module and returns it's environment as a frozen table.
function Module:load()
	self.func()
	return self.env
end

-- Call a function within module environment. This function returns false if
-- fname doesn't exist.
function Module:call(fname, ...)
	local f = self.env[fname]
	if type(f) == "function" then
		setfenv(f, self.env)(...)
		return true
	end
	return false
end

