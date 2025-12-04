package.preload["fs"] = function()
	local fs = {
		File = {},
		os = {},
		stdio = {},
	}

	fs.File.__index = fs.File
	function fs.File.from_fd(fd, position)
		return setmetatable({
			fd = fd,
			position = position or 0,
		}, fs.File)
	end

	--- Read up to `size` bytes at position `pos`, store them in given buffer
	--- and return the number of bytes read.
	function fs.File:pread(io, pos, buf, size)
		local ptr, len = buf:reserve(size or 4096)
		io:pread(self.fd, ptr, len, pos)
		local ok, read = coroutine.yield()
		if not ok then error(read) end
		buf:commit(read)
		return read
	end

	--- Read up to `size` bytes, store them in given buffer and return the
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

	--- Read at least `min` bytes, store them in given buffer and return the
	--- total number of bytes read.
	function fs.File:read_at_least(io, buf, min)
		local total_read = 0
		while total_read < min do
			local read = self:read(io, buf, min - total_read)
			total_read = total_read + read
			if read == 0 then break end
		end

		return total_read
	end

	--- Close file, rendering it unusable for I/O.
	function fs.File:close(io)
		io:close(self.fd)
		local ok, read = coroutine.yield()
		if not ok then error(read) end
		return read
	end

	--- Get information about file.
	function fs.File:stat(io)
		io:fstat(self.fd)
		local ok, stat = coroutine.yield()
		if not ok then error(stat) end
		return stat
	end

	--- Read entire file, store bytes in given buffer and return number of bytes
	--- read.
	function fs.File:read_all(io, buf)
		local stat = self:stat(io)
		return self:read_at_least(io, buf, stat.size)
	end

	--- Synchronize changes to a file.
	function fs.File:stat(io)
		io:fsync(self.fd)
		local ok, err = coroutine.yield()
		if not ok then error(err) end
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

	--- Remove file at `path`.
	function fs.os.remove(io, path, remove_dir)
		io:unlinkat(at_fdcwd, path, remove_dir or false)
		local ok, err = coroutine.yield()
		if not ok then error(err) end
	end

	fs.stdio.input = fs.File.from_fd(0, -1)
	fs.stdio.output = fs.File.from_fd(1, -1)
	fs.stdio.error = fs.File.from_fd(2, -1)

	return fs
end
