local io = require("io")
local math = require("math")
local string = require("string")
local t = require("test")

t.test("BufReader default buffer size is io.default_buffer_size", function()
	local reader = {}
	local buf_reader = io.BufReader.new(reader)
	assert(buf_reader:available() == io.default_buffer_size)
	local _, len = buf_reader.buffer:reserve(0)
	assert(len == io.default_buffer_size)
end)

t.test("BufReader buffer uses at least buffer of size", function()
	local reader = {}
	local buf_reader = io.BufReader.new(reader, { size = 128 })
	assert(buf_reader:available() == 128)
	local _, len = buf_reader.buffer:reserve(0)
	assert(len == 128)
end)

t.test(
	"multiple read from BufReader using 32 bytes and single byte reads",
	function()
		local reader_read_call = 0
		local reader = {
			read = function(_, buf)
				reader_read_call = reader_read_call + 1
				local copied =
					string.buffer.copy(string.random(io.default_buffer_size), buf)
				buf:commit(copied)
				return copied
			end,
		}

		local buf_reader = io.BufReader.new(reader)
		local buf = string.buffer.new(32) -- 32 is minimum size of buffer.

		-- Read using 32 bytes buffer.
		local read = buf_reader:read(buf)
		assert(read == #buf)
		assert(read == 32)
		assert(reader_read_call == 1)
		assert(buf_reader:buffered() == io.default_buffer_size - 32)

		-- Read all buffered data byte per byte.
		while buf_reader:buffered() > 1 do
			buf:reset()
			buf:commit(31)
			read = buf_reader:read(buf)
			assert(read == 1)
			assert(reader_read_call == 1)
		end

		-- Read another 32 bytes. This should trigger another read.
		buf:reset()
		read = buf_reader:read(buf)
		assert(read == 32)
		assert(reader_read_call == 2)
		-- -31 instead of -32 as buf_reader internal buffer contained a last byte.
		assert(buf_reader:buffered() == io.default_buffer_size - 31)
	end
)

t.test(
	"read from BufReader using bigger buffer than BufReader internal buffer",
	function()
		local reader_read_call = 0
		local reader = {
			read = function(_, buf)
				reader_read_call = reader_read_call + 1
				local copied =
					string.buffer.copy(string.random(io.default_buffer_size), buf)
				buf:commit(copied)
				return copied
			end,
		}

		local buf_reader = io.BufReader.new(reader, { size = 32 })
		local buf = string.buffer.new(64)

		-- Short circuit buffer and directly read into buf.
		local read = buf_reader:read(buf)
		assert(read == #buf)
		assert(read == 64)
		assert(reader_read_call == 1)
		assert(buf_reader:buffered() == 0)

		buf:reset()
		buf:commit(33) -- 31 bytes available in buf.

		-- Read into internal buffer then copy 31 bytes to buf.
		read = buf_reader:read(buf)
		assert(read == #buf - 33)
		assert(read == 64 - 33)
		assert(reader_read_call == 2)
		assert(buf_reader:buffered() == 1)

		-- Copy from internal buffer then read directly into buf.
		buf = string.buffer.new(32)
		local buffered_byte = buf_reader.buffer:tostring()
		read = buf_reader:read(buf)
		assert(read == #buf)
		assert(read == 32)
		assert(reader_read_call == 3)
		assert(buf_reader:buffered() == 1)
		-- Ensure buffered byte has been read.
		assert(buf:get(1) == buffered_byte)
	end
)
