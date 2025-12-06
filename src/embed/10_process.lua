package.preload["process"] = function()
	local fs = require("fs")

	local process = {
		Child = {},
		stdin = fs.File.from_fd(0, -1),
		stdout = fs.File.from_fd(1, -1),
		stderr = fs.File.from_fd(2, -1),
	}
	process.Child.__index = process.Child

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

	--- Spawn a new child process.
	function process.spawn(io, args, options)
		options = options or {}

		io:spawn(
			args,
			options.env_vars or {},
			options.stdin or "inherit",
			options.stdout or "inherit",
			options.stderr or "inherit"
		)
		local ok, result = coroutine.yield()
		if not ok then error(result) end

		if options.stdin ~= "pipe" then
			result.stdin = nil
		else
			result.stdin = fs.File.from_fd(result.stdin)
		end
		if options.stdout ~= "pipe" then
			result.stdout = nil
		else
			result.stdout = fs.File.from_fd(result.stdout)
		end
		if options.stderr ~= "pipe" then
			result.stderr = nil
		else
			result.stderr = fs.File.from_fd(result.stderr)
		end

		return setmetatable(result, process.Child)
	end

	--- Wait until process exits and return its exit code.
	function process.Child:wait(io)
		io:waitpid(self.pid)
		local ok, status = coroutine.yield()
		if not ok then error(status) end
		self.status = status
		return status
	end

	return process
end
