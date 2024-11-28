local io = require("io")
local string = require("string")
local t = require("test")

t.test("BufWriter default buffer size is io.default_buffer_size", function()
	local writer = {}
	local buf_writer = io.BufReader.new(writer)
	assert(buf_writer:available() == io.default_buffer_size)
	local _, len = buf_writer.buffer:reserve(0)
	assert(len == io.default_buffer_size)
end)

t.test("BufWriter buffer uses at least buffer of size", function()
	local writer = {}
	local buf_writer = io.BufReader.new(writer, { size = 128 })
	assert(buf_writer:available() == 128)
	local _, len = buf_writer.buffer:reserve(0)
	assert(len == 128)
end)

t.test("multiple write to BufWriter", function()
	local writer_write_call = 0
	local latest_write_size = 0
	local writer = {
		write = function(_, buf)
			writer_write_call = writer_write_call + 1
			latest_write_size = #buf
			return #buf
		end,
	}

	local buf_writer = io.BufWriter.new(writer)

	local write = buf_writer:write("foo")
	assert(write == 3)
	assert(buf_writer:buffered() == write)
	assert(writer_write_call == 0)
	assert(latest_write_size == 0)

	write = buf_writer:write(string.random(io.default_buffer_size))
	assert(write == io.default_buffer_size)
	assert(buf_writer:buffered() == write)
	assert(writer_write_call == 1)
	assert(latest_write_size == 3)

	write = buf_writer:write("foo")
	assert(write == 3)
	assert(buf_writer:buffered() == write)
	assert(writer_write_call == 2)
	assert(latest_write_size == io.default_buffer_size)

	-- Flush buffered data.
	buf_writer:flush()
	assert(buf_writer:buffered() == 0)
	assert(writer_write_call == 3)
	assert(latest_write_size == 3)

	-- Flush empty buffer doesn't call writer.
	buf_writer:flush()
	assert(buf_writer:buffered() == 0)
	assert(writer_write_call == 3)
	assert(latest_write_size == 3)
end)

t.test("BufWriter:flush writes until internal buffer is empty", function()
	local writer_write_call = 0
	local writer = {
		write = function(_, _buf)
			writer_write_call = writer_write_call + 1
			return 1
		end,
	}

	local buf_writer = io.BufWriter.new(writer)

	local write = buf_writer:write(string.random(io.default_buffer_size))
	assert(write == io.default_buffer_size)
	assert(buf_writer:buffered() == io.default_buffer_size)

	buf_writer:flush()
	assert(writer_write_call == io.default_buffer_size)
	assert(buf_writer:buffered() == 0)

	-- Write directly to internal writer as buffer is larger than internal buffer?
	write = buf_writer:write(string.random(2 * io.default_buffer_size))
	assert(write == 1)
	assert(buf_writer:buffered() == 0)
	assert(writer_write_call == io.default_buffer_size + 1)
end)
