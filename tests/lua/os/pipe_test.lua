local os = require("os")
local t = require("test")
local time = require("time")

t.test("read and write to pipe from different goroutines", function()
	local reader, writer = os.pipe()

	go(function()
		writer:write_string("Hello world!")
		writer:close()
	end)

	local content = reader:read_to_end()
	t.assert_eq(content, "Hello world!")
end)

t.test("closing writer unblock waiting reader", function()
	local reader, writer = os.pipe()

	go(function()
		time.sleep(10 * time.millisecond)
		writer:close()
	end)

	local now = time.Instant.now()
	local content = reader:read_to_end()
	assert(now:elapsed() >= 10 * time.millisecond)
end)

t.test("pipe are buffered by default", function()
	local reader, writer = os.pipe()

	writer:write_string("Hello world!")
	writer:close()

	-- read_line is only defined on io.BufReader.
	local content = reader:read_line()

	t.assert_eq(content, "Hello world!")
end)

t.test("pipe with buffer size of 0 are not buffered", function()
	local reader, _writer = os.pipe(0, 0)

	assert(reader.read_line == nil)
end)
