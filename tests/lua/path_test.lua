local path = require("path")
local t = require("test")

local fpath = package.meta.path
local fdir = path.parent(fpath)
local fname = path.file_name(fpath)

t.test("canonicalize path returns absolute path", function()
	local rel_fpath = fdir .. "/../" .. path.file_name(fdir) .. "/" .. fname
	assert(path.canonicalize(rel_fpath) == fpath)
end)
