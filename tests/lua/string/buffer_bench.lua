local string = require("string")
local t = require("test")

t.bench("read/write 1024 bytes to string.Buffer", function(b)
	local buf = string.Buffer.new()
	local str = ("!"):rep(1024)

	for _ = 1, b.n do
		buf:write_string(str)
		buf:read_to_end()
	end
end)
