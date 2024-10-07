local os = require("os")

-- Echo stdin.
do
	-- Open stdin with read permissions.
	local stdin = os.File.open("/proc/self/fd/0", "r")

	while true do
		local line = stdin:read_line()
		if not line then break end
		print(line)
	end

	stdin:close()
end

-- Read an entire file.
do
	local txt = os.File.read("examples/file.lua")
	print(txt)
end
