return function(main_path, resolve_path, list_files, caller_source)
	local package = require("package")
	local os = require("os")
	local path = require("path")
	local table = require("table")
	local string = require("string")

	local M = package

	-- package.meta table.
	do
		M.meta = {}
		setmetatable(M.meta, {
			__index = function(_, k)
				if k == "path" then
					return caller_source(0)
				elseif k == "main" then
					return main_path
				end

				return nil
			end,
		})
	end

	-- Remove path and cpath in favor of homemade searchers.
	package.path = ""
	package.cpath = ""

	-- Table of loaded files to avoid loading a file multiple times.
	local file_loaded = {}
	-- Table of loaded packages to avoid loading a package multiple times.
	local pkg_loaded = {}

	-- Load ith file in files using env.
	function load_file(files, i, env)
		local fpath = files[i]

		-- Prevent multiple loading of file.
		if file_loaded[fpath] then return env end
		file_loaded[fpath] = true

		-- Create package environment.
		if not env then
			env = {}
			setmetatable(env, {
				__index = function(t, k)
					-- key not found in env, it may be global or it may be part of another
					-- file. Let's try to load other files first.
					local j = i + 1
					while rawget(t, k) == nil and j <= #files do
						load_file(files, j, env)
						j = j + 1
					end
					if rawget(t, k) ~= nil then return rawget(t, k) end

					-- Prevent package to modify global environment.
					if k == "_G" then return t end

					return _G[k]
				end,
			})
		end

		local chunk = loadfile(fpath, nil, env)
		if not chunk then error(("failed to load file '%s'"):format(fpath)) end
		chunk()

		return env
	end

	local function file_searcher(pkgname)
		local pkgpath = resolve_path(pkgname)

		local ok, files = pcall(list_files, pkgpath)
		if not ok then
			local err = files
			if type(err) == "io.Error" and err.kind == "not_found" then
				error("failed to find " .. pkgname, { cause = err })
			else
				error(err)
			end
		end

		return function()
			if pkg_loaded[pkgpath] then return pkg_loaded[pkgpath] end

			local env = nil
			for i in ipairs(files) do
				env = load_file(files, i, env)
			end

			pkg_loaded[pkgpath] = freeze(env)

			return pkg_loaded[pkgpath]
		end,
			pkgpath
	end

	package.loaders = {
		package.searchers[1], -- Preload loader.
		file_searcher,
	}
	package.searchers = package.loaders
end
