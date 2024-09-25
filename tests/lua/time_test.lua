local t = require("test")
local time = require("time")

t.test(
	"duration arithmetic 1s + 1s == 2 * 1s",
	function() assert(time.second + time.second == 2 * time.second) end
)

t.test(
	"duration arithmetic 1s - 1s == 0 * 1s",
	function() assert(time.second - time.second == 0 * time.second) end
)

t.test("duration unary minus (-1s) is not supported", function()
	local ok, err = pcall(function() print(-time.second) end)

	assert(not ok, "no error thrown")
	assert(
		string.contains(
			err,
			"attempt to perform arithmetic on field 'second' (a userdata value)"
		)
	)
end)

t.test(
	"tostring(time.hour) is 1h",
	function() t.assert_eq(tostring(time.hour), "1h") end
)

t.test(
	"tostring(time.minute) is 1m",
	function() t.assert_eq(tostring(time.minute), "1m") end
)

t.test(
	"tostring(time.second) is 1s",
	function() t.assert_eq(tostring(time.second), "1s") end
)

t.test(
	"tostring(time.second + time.nanosecond) is 1s",
	function() t.assert_eq(tostring(time.second + time.nanosecond), "1s") end
)

t.test(
	"tostring(time.second + time.microsecond) is 1s",
	function() t.assert_eq(tostring(time.second + time.microsecond), "1s") end
)

t.test(
	"tostring(time.second + time.millisecond) is 1.001s",
	function() t.assert_eq(tostring(time.second + time.millisecond), "1.001s") end
)

t.test(
	"tostring(time.second + 10 * time.millisecond) is 1.01s",
	function() t.assert_eq(tostring(time.second + 10 * time.millisecond), "1.01s") end
)

t.test(
	"tostring(time.second + 100 * time.millisecond) is 1.01s",
	function() t.assert_eq(tostring(time.second + 100 * time.millisecond), "1.1s") end
)

t.test(
	"tostring(time.millisecond) is 1ms",
	function() t.assert_eq(tostring(time.millisecond), "1ms") end
)

t.test(
	"tostring(time.millisecond) is 1µs",
	function() t.assert_eq(tostring(time.microsecond), "1µs") end
)

t.test(
	"tostring(time.millisecond) is 1ns",
	function() t.assert_eq(tostring(time.nanosecond), "1ns") end
)

t.test(
	"tostring(time.millisecond + time.microsecond) is 1.001ms",
	function()
		t.assert_eq(tostring(time.millisecond + time.microsecond), "1.001ms")
	end
)

t.test(
	"tostring(time.millisecond + time.microsecond + time.nanosecond) is 1.001001ms",
	function()
		t.assert_eq(
			tostring(time.millisecond + time.microsecond + time.nanosecond),
			"1.001001ms"
		)
	end
)

t.test(
	"tostring(time.millisecond + time.nanosecond) is 1.000001ms",
	function()
		t.assert_eq(tostring(time.millisecond + time.nanosecond), "1.000001ms")
	end
)

t.test("time.after(time.millisecond) sends nil after 1ms", function()
	local dur = 5 * time.millisecond
	local rx = time.after(dur)
	local now = time.Instant.now()
	local v = rx:recv()
	local elapsed = now:elapsed()

	t.assert_eq(v, nil)
	assert(
		elapsed >= dur and elapsed < (2 * dur),
		"elapsed duration exceed expectation: " .. tostring(elapsed)
	)
end)
