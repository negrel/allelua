local t = require("test")
local time = require("time")

t.test("duration arithmetic 1s + 1s == 2 * 1s", function()
	assert(time.second + time.second == 2 * time.second)
end)

t.test("duration arithmetic 1s - 1s == 0 * 1s", function()
	assert(time.second - time.second == 0 * time.second)
end)

t.test("duration unary minus (-1s) is not supported", function()
	local _, err = pcall(function()
		print(-time.second)
	end)

	assert(string.contains(err, "attempt to perform arithmetic on field 'second' (a userdata value)"))
end)
