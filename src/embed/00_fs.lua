package.preload["fs"] = function()
	local File = {}
	File.__index = File
	function File:close(io)
		io:close(self.fd)
		coroutine.yield()
	end

	return {
		os = {
			File = File,
			open = function(io, path, options, mode)
				io.openat(io, -100, path, options or {}, mode or 0)
				local ok, fd = coroutine.yield()
				if not ok then error(fd) end

				return setmetatable({ fd = fd }, File)
			end
		},
	}
end
