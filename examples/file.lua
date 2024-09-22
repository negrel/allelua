local os = require("os")

-- Open stdin with read permissions.
local stdin = os.File.open("/proc/self/fd/0", "r")

local buf = ""
while true do
	local byte = stdin:read_exact(1)
	if byte == "\n" then
		print(buf)
		buf = ""
	else
		buf = buf .. byte
	end
end
