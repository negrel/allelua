local os = require("os")
local package = require("package")
local path = require("path")
local string = require("string")
local t = require("test")

local parent_dir = path.parent(package.meta.path)
local file_txt = path.join(parent_dir, "testdata/file.txt")

t.test("piped tr [a-z] [A-Z]", function()
	local buf = string.buffer.new()

	local proc = os.exec("tr", {
		args = { "[a-z]", "[A-Z]" },
		stdin = "piped",
		stdout = "piped",
	})

	buf:put("hello from lua code")
	local len = #buf

	proc.stdin:write_all(buf)
	proc.stdin:close()

	proc.stdout:read(buf, len)

	assert("HELLO FROM LUA CODE" == tostring(buf))

	-- Process terminate.
	local status = proc:wait()
	assert(status.success)
end)

t.test("pass file as stdin of cat", function()
	local f = os.File.open(file_txt, { read = true, buffer_size = 0 })
	local f_content = f:read_to_end()

	-- Seek beginning of file.
	f:rewind()

	-- Exec cat with f as stdin.
	local proc = os.exec("cat", { stdin = f, stdout = "piped" })
	print("proc started...")

	-- Read stdout.
	local content = proc.stdout:read_to_end()

	assert(content == f_content, "stdout content differ from expected")

	-- Process terminate.
	local status = proc:wait()
	assert(status.success)
end)

t.test("pass closed file as stdin of process fails", function()
	local f = os.File.open(file_txt, { read = true })
	f:close()

	-- Exec cat with f as stdin.
	local ok, err = pcall(os.exec, "cat", { stdin = f, stdout = "piped" })
	assert(not ok and err.kind == "Closed")
end)

t.test("process pipes are buffered by default", function()
	-- Exec cat with f as stdin.
	local proc = os.exec("cat", { stdin = "piped", stdout = "piped" })

	proc.stdin:write_string("Hello world!")
	proc.stdin:close()

	-- read_line is only defined on io.BufReader.
	local content = proc.stdout:read_line()

	t.assert_eq(content, "Hello world!")

	-- Process terminate.
	local status = proc:wait()
	assert(status.success)
end)

t.test("process pipes with buffer size of 0 are not buffered", function()
	-- Exec cat with f as stdin.
	local proc = os.exec(
		"cat",
		{ stdin = "null", stdout = { from = "piped", buffer_size = 0 } }
	)

	assert(proc.stdout.read_line == nil)

	-- Process terminate.
	local status = proc:wait()
	assert(status.success)
end)

t.test("process returns non zero exit code", function()
	local proc = os.exec("ls", { args = { "/dir/that/doesn't/exist" } })
	local status = proc:wait()

	assert(not status.success, "process should fail but didn't")
	assert(status.code == 2, "process should exit with status code 2")
end)
