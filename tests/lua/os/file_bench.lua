local buffer = require("string.buffer")
local os = require("os")
local package = require("package")
local path = require("path")
local t = require("test")

local parent_dir = path.parent(package.meta.path)
local file_txt = path.join(parent_dir, "testdata/file.txt")

t.bench("read file line by line", function(b)
	for _i = 1, b.n do
		local f = os.File.open(file_txt, { read = true })
		while true do
			local line = f:read_line()
			if not line then break end
		end
		f:close()
	end
end)

t.bench("read file to end using io.Reader:read_to_end()", function(b)
	for _i = 1, b.n do
		local f = os.File.open(file_txt, { read = true })
		f:read_to_end()
		f:close()
	end
end)

t.bench(
	"read file to end using io.Reader:read() and a LuaJIT buffer",
	function(b)
		for _i = 1, b.n do
			local f = os.File.open(file_txt, { read = true })
			local len = f:metadata().len
			local buf = buffer.new(len)
			f:read(buf, len)
			f:close()
		end
	end
)
