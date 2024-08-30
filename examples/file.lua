local fs = require("fs")

-- Open stdin with read permissions.
local stdin = fs.file.open("/proc/self/fd/0", "r")

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

