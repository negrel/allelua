package.preload["process"] = function()
	local fs = require("fs")

	local process = {
		stdin = fs.File.from_fd(0, -1),
		stdout = fs.File.from_fd(1, -1),
		stderr = fs.File.from_fd(2, -1),
	}

	--- Retrieve process current working directory.
	function process.getcwd(io, buf)
		local ptr, len = buf:reserve(fs.path_max)
		io:getcwd(ptr, len)
		local ok, str = coroutine.yield()
		if not ok then error(str) end
		buf:commit(#str)
		return #str
	end

	--- Change process working directory.
	function process.chdir(io, buf)
		io:chdir(buf, #buf)
		local ok, err = coroutine.yield()
		if not ok then error(err) end
	end

	return process
end
