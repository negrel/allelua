return function(main_path, path_canonicalize, caller_source)
	local package = require("package")
	local os = require("os")
	local path = require("path")
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

	local file_loaded = {} -- file_searcher loaded cache table.
	local function file_searcher(modname)
		local fpath = modname
		if string.has_prefix(fpath, "@/") then -- relative to current working dir.
			fpath = path.join(os.current_dir(), string.slice(fpath, 3))
		elseif path.is_relative(fpath) then -- relative to current file.
			fpath = path.join(path.parent(caller_source(2)), fpath)
		end

		local ok, fpath = pcall(path_canonicalize, fpath)
		if not ok then
			local err = fpath
			if type(err) == "io.Error" and err.kind == "NotFound" then
				error("failed to find " .. modname, { cause = err })
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
