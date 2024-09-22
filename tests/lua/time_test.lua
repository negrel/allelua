local t = require("test")
local time = require("time")

t.test("duration arithmetic 1s + 1s == 2 * 1s", function()
	assert(time.second + time.second == 2 * time.second)
end)

t.test("duration arithmetic 1s - 1s == 0 * 1s", function()
	assert(time.second - time.second == 0 * time.second)
end)

t.test("duration unary minus (-1s) is not supported", function()
	local ok, err = pcall(function()
		print(-time.second)
	end)

	assert(not ok, "no error thrown")
	assert(string.contains(err, "attempt to perform arithmetic on field 'second' (a userdata value)"))
end)

t.test("time.after(time.millisecond) sends nil after 1ms", function()
	local dur = 5 * time.millisecond
	local rx = time.after(dur)
	local now = time.Instant.now()
	local v = rx:recv()
	local elapsed = now:elapsed()

	t.assert_eq(v, nil)
	assert(elapsed >= dur and elapsed < (2 * dur), "elapsed duration exceed expectation: " .. tostring(elapsed))
end)
