local string = require("string")
local t = require("test")

t.test("write_string and read_to_end from string.Buffer", function()
	local buf = string.Buffer.new()

	buf:write_string("foo")
	assert(buf:read_to_end() == "foo")
end)

t.test("write and read from string.Buffer", function()
	local input = string.buffer.new()
	input:put("foo")

	local buf = string.Buffer.new()
	buf:write(input)

	local output = string.buffer.new(#buf)
	buf:read(output)
	assert(output:tostring() == "foo")
end)

t.test("read_from a string.Buffer to another", function()
	local a = string.Buffer.new()
	local b = string.Buffer.new()

	a:write_string("foo")
	b:read_from(a)
	assert(b:read_to_end() == "foo")
end)
