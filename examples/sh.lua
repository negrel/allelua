local os = require("os")
local package = require("package")

coroutine.nursery(function(go)
	local sh = require("sh").new(go)

	local f = os.File.open(package.meta.path, { read = true })

	print(sh.tr({ stdin = f }, "[a-z]", "[A-Z]"):tr('"', "'"):output())
end)
