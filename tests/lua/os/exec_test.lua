local os = require("os")
local string = require("string")
local t = require("test")

t.test("piped tr [a-z] [A-Z]", function()
	local buf = string.buffer.new()

	local proc = os.exec("tr", {
		args = { "[a-z]", "[A-Z]" },
		stdin = "piped",
		stdout = "piped",
	})
	local stdin = proc:stdin()
	local stdout = proc:stdout()

	buf:put("hello from lua code")
	local len = #buf

	stdin:write_all(buf)
	stdin:close()

	stdout:read(buf, len)

	assert("HELLO FROM LUA CODE" == tostring(buf))

	-- Process terminate.
	local status = proc:wait()
	assert(status.success)
end)
