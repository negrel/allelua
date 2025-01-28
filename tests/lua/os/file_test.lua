local math = require("math")
local os = require("os")
local path = require("path")
local t = require("test")

local function temp_path()
	local path = path.join(os.temp_dir(), "allelua_" .. tostring(math.random()))
	pcall(os.remove_file, path)
	return path
end

t.test("close an already closed file", function()
	local stdin = os.File.open("/proc/self/fd/0", { read = true })

	-- First close throw no error.
	stdin:close()

	-- Second close throw a Closed error.
	local ok, err = pcall(stdin.close, stdin)
	assert(
		not ok and err.kind == "closed",
		"received a non closed error on double close"
	)
end)

t.test("closing file flush content", function()
	local tmp_path = temp_path()

	local f = os.File.open(tmp_path, { create_new = true, write = true })
	f:write_string("Hello world!")

	-- Close file.
	f:close()

	-- File is NOT empty.
	local content = os.File.read(tmp_path)
	assert(content == "Hello world!")
end)

t.test("seek to beginning of file to read written data", function()
	local tmp_path = temp_path()

	local f = os.File.open(
		tmp_path,
		{ create_new = true, read = true, write = true, buffer_size = 0 }
	)
	f:write_string("Hello world!")

	local content = f:read_to_end()
	assert(content == "")

	-- Seek to beginning of file.
	f:rewind()

	-- Content is available again.
	content = f:read_to_end()
	assert(content == "Hello world!")

	-- Close file.
	f:close()
end)

t.test("open file for unbuffered I/O", function()
	local tmp_path = temp_path()
	-- Buffer size of 0 means unbuffered I/O.
	local f = os.File.open(
		tmp_path,
		{ create_new = true, read = true, write = true, buffer_size = 0 }
	)
	f:close()

	f = os.File.open(tmp_path, { read = true, write = true })
	f:close()
end)

t.test("listing directory entries", function()
	local parent_dir = path.parent(package.meta.path)
	local dir = path.join(parent_dir, "testdata/dir")

	local expected = {
		{ file_name = "entry1" },
		{ file_name = "entry2" },
	}

	local actual = {}
	for f in os.Dir.iterator(dir) do
		assert(f.path:has_suffix(f.file_name))
		table.push(actual, { file_name = f.file_name })
	end

	assert(
		table.deep_eq(expected, actual),
		"entries of directory doesn't match expected"
	)
end)
