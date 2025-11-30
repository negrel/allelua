package.preload["fs"] = function()
	local fs = {
		File = {},
		os = {}
	}

	fs.File.__index = fs.File
	function fs.File.from_fd(fd)
		return setmetatable({
			fd = fd,
			position = 0,
		}, fs.File)
	end

	--- Read up to `size` bytes at position `pos` and store them in given buffer
	--- and return the number of bytes read.
	function fs.File:pread(io, pos, buf, size)
		local ptr, len = buf:reserve(size or 4096)
		io:pread(self.fd, ptr, len, pos)
		local ok, read = coroutine.yield()
		if not ok then error(read) end
		buf:commit(read)
		return read
	end

	--- Read up to `size` bytes and store them in given buffer and return the
	--- number of bytes read.
	function fs.File:read(io, buf, size)
		local read = fs.File.pread(self, io, self.position, buf, size)
		self.position = self.position + read
		return read
	end

	--- Write `#buf` bytes at position pos in the file and return the number of
	--- bytes written.
	function fs.File:pwrite(io, pos, buf)
		local ptr, len = buf:ref()
		io:pwrite(self.fd, ptr, len, pos)
		local ok, write = coroutine.yield()
		if not ok then error(write) end
		return write
	end

	--- Write `#buf` bytes to the file and return the number of bytes written.
	function fs.File:write(io, buf)
		local write = fs.File.pwrite(self, io, self.position, buf)
		self.position = self.position + write
		return write
	end

	--- Close file, rendering it unusable for I/O.
	function fs.File:close(io)
		io:close(self.fd)
		local ok, read = coroutine.yield()
		if not ok then error(read) end
		return read
	end

	local function fmodifier(mode)
		local all = {
			read = mode.read == true or false,
			write = mode.write == true or false,
			execute = mode.execute == true or false,
		}
		local user = mode.user or all
		local group = mode.group or all
		local other = mode.other or all

		local result = 0

		if user.read == true then
			result = result + 256 -- 0o400
		end
		if user.write == true then
			result = result + 128 -- 0o200
		end
		if user.execute == true then
			result = result + 64 -- 0o100
		end

		if group.read == true then
			result = result + 32 -- 0o40
		end
		if group.write == true then
			result = result + 16 -- 0o20
		end
		if group.execute == true then
			result = result + 8 -- 0o10
		end

		if other.read == true then
			result = result + 4 -- 0o4
		end
		if other.write == true then
			result = result + 2 -- 0o2
		end
		if other.execute == true then
			result = result + 1 -- 0o1
		end

		return result
	end
	local mode_perm = { read = true, write = true }

	--- Open file at `path`.
	function fs.os.open(io, path, options, mode)
		io:openat(at_fdcwd, path, options or {}, fmodifier(mode or mode_perm))
		local ok, fd = coroutine.yield()
		if not ok then error(fd) end

		return fs.File.from_fd(fd)
	end

	return fs
end
