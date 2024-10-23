local io = require("io")
local string = require("string")
local t = require("test")

t.test("single write in io.PipeWriter blocks until data is read", function()
	local writer, reader = io.pipe()

	local reader_done = false

	coroutine.nursery(function(go)
		-- Reader
		go(function()
			local buf = string.buffer.new(1024)
			reader:read(buf)
			assert(buf:tostring() == "Hello world!")
			reader_done = true
		end)

		-- Writer
		do
			local buf = string.buffer.new()
			buf:put("Hello world!")
			writer:write(buf)
		end
	end)
	assert(reader_done)
end)

t.test(
	"single write in io.PipeWriter blocks until multiple smaller read are done",
	function()
		local writer, reader = io.pipe()

		local reader_done = false

		coroutine.nursery(function(go)
			-- Reader
			go(function()
				local buf = string.buffer.new()
				for i = 1, 3 do
					reader:read(buf, 1)
				end
				assert(buf:tostring() == "foo")
				reader_done = true
			end)

			-- Writer
			do
				local buf = string.buffer.new()
				buf:put("foo")
				writer:write(buf)
				assert(tostring(buf) == "")
			end
		end)
		assert(reader_done)
	end
)

t.test("read returns error on closed pipe", function()
	local writer, reader = io.pipe()

	coroutine.nursery(function(go)
		go(writer.close, writer)

		local ok, err = pcall(reader.read, reader, string.buffer.new())
		assert(not ok and err:is(io.errors.closed))
	end)
end)

t.test("write returns error on closed pipe", function()
	local writer = io.pipe()

	coroutine.nursery(function(go)
		go(writer.close, writer)

		local buf = string.buffer.new()
		buf:put("foo")
		local ok, err = pcall(writer.write, writer, buf)
		assert(not ok and err:is(io.errors.closed))
	end)
end)

t.test("reader:read_to_end() reads until pipe is closed", function()
	local writer, reader = io.pipe()

	coroutine.nursery(function(go)
		go(function()
			local buf = string.buffer.new()
			buf:put("foo")
			writer:write(buf)
			buf:put("bar")
			writer:write(buf)
			writer:close()
		end)

		local content = reader:read_to_end()
		assert(content == "foobar")
	end)
end)
