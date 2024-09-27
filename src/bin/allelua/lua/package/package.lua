return function(main_path, path_canonicalize)
	local package = require("package")
	local table = require("table")
	local env = require("env")
	local path = require("path")

	local M = package

	-- Remove coroutine, table.new and table.clear module.
	package.loaded.coroutine = nil
	package.loaded["table.new"] = nil
	package.loaded["table.clear"] = nil

	-- Remove path and cpath in favor or home made searchers.
	package.path = ""
	package.cpath = ""

	-- Add meta table.
	M.meta = table.freeze { path = main_path, main = true }

	local file_loaded = {} -- file_searcher loaded cache table.
	local function file_searcher(modname)

		local fpath = modname
		if string.has_prefix(fpath, "@/") then -- relative to current working dir.
			fpath = path.join(env.current_dir(), string.slice(fpath, 3))
		elseif path.is_relative(fpath) then -- relative to current file.
			fpath = path.join(path.parent(M.meta.path), fpath)
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
			return table.freeze(result)
		end, fpath
	end

	package.loaders = {
		package.searchers[1], -- Preload loader.
		file_searcher,
	}
	package.searchers = package.loaders
end
