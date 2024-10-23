local t = require("test")

t.test("std module are available in _G", function()
	local stdmodules = {
		"error",
		"io",
		"os",
		"package",
		"path",
		"sh",
		"string",
		"sync",
		"table",
		-- "test",
		"time",
	}

	for _, m in pairs(stdmodules) do
		assert(_G[m], ("%s is not exposed in _G"):format(m))
	end
end)
