return function(main_path, path_canonicalize, caller_source)
	local package = require("package")
	local env = require("env")
	local path = require("path")

	local M = package

	-- Remove coroutine.
	package.loaded.coroutine = nil

	-- Remove path and cpath in favor of homemade searchers.
	package.path = ""
	package.cpath = ""

	local file_loaded = {} -- file_searcher loaded cache table.
	local function file_searcher(modname)
		local fpath = modname
		if string.has_prefix(fpath, "@/") then -- relative to current working dir.
			fpath = path.join(env.current_dir(), string.slice(fpath, 3))
		elseif path.is_relative(fpath) then -- relative to current file.
			fpath = path.join(path.parent(caller_source(2)), fpath)
		end

		local ok, err = pcall(function()
			fpath = path_canonicalize(fpath)
		end)
		if not ok then
			if err.kind == "NotFound" then
				error("failed to find " .. modname)
			else
				error(err)
			end
		end

		return function()
			if file_loaded[fpath] then return file_loaded[fpath] end
			local result = dofile(fpath)
			file_loaded[fpath] = result
			return freeze(result)
		end,
			fpath
	end

	package.loaders = {
		package.searchers[1], -- Preload loader.
		file_searcher,
	}
	package.searchers = package.loaders
end
