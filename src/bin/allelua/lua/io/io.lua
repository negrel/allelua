return function(M)
	local math = require("math")
	local sync = require("sync")
	local error = require("error")
	local buffer = require("string.buffer")

	M.copy = function(reader, writer, opts)
		opts = opts or {}
		opts.flush = opts.flush or false
		opts.close = opts.close or false

		if not reader then error("reader is nil") end
		if not writer then error("writer is nil") end

		local total = 0

		if rawtype(reader.write_to) == "function" then
			local ok, write = pcall(reader.write_to, reader, writer)
			if not ok then
				local err = write
				if not err:is(M.errors.closed) then error(err) end
			end

			if opts.flush then writer:flush() end
			total = write
		else
			local buf = buffer.new()
			while true do
				-- Read into buffer.
				local ok, read = pcall(reader.read, reader, buf, 4096)
				if not ok then
					local err = read
					if err:is(M.errors.closed) then break end
					error(err)
				end
				if read == 0 then break end

				-- Write from buffer.
				local ok, write = pcall(writer.write_all, writer, buf)
				if not ok then
					local err = write
					if err:is(M.errors.closed) then break end
					error(err)
				end

				if opts.flush then writer:flush() end
				total = total + write
			end
		end

		if opts.close then
			if rawtype(reader.close) == "function" then
				pcall(reader.close, reader)
			end
			if rawtype(writer.close) == "function" then
				pcall(writer.close, writer)
			end
		end

		return total
	end

	function M.read_to_end(self)
		local buf_size = M.default_buffer_size
		local free_buf_size = buf_size
		local buf = buffer.new(buf_size)

		while true do
			local ok, read = pcall(self.read, self, buf)
			if not ok then
				local err = read
				if err:is(M.errors.closed) then break end
				error(err)
			end
			if read == 0 then break end
			-- buffer is full, reserve more space.
			if read == free_buf_size then
				buf:reserve(free_buf_size)
				buf_size = buf_size * 2
				free_buf_size = buf_size - #buf
			end
		end

		return buf:tostring()
	end

	function M.write_all(self, buf, edit_buf)
		local write = self:write(buf)
		local len = #buf

		-- Multiple write call needed.
		if write ~= len then
			if edit_buf then
				buf:skip(write)
				while write < len do
					local w = self:write(buf)
					buf:skip(w)
					write = write + w
				end
			else
				local clone = buffer.new()
				local ptr = buf:ref()
				clone:putcdata(ptr + write, len - write)
				while write < len do
					local w = self:write(clone)
					clone:skip(w)
					write = write + w
				end
			end
		end

		return len
	end

	function M.read_all(self, buf)
		local read = 0
		local len = #buf
		while read < len do
			read = read + self:read(buf)
		end

		return len
	end

	function M.write_string(writer, str)
		local buf = buffer.new(#str)
		buf:put(str)
		return M.write_all(writer, buf, true)
	end

	local seek_from_beginning = M.SeekFrom.start(0)
	function M.rewind(self)
		self:seek(seek_from_beginning)
	end

	M.PipeReader = { __type = "io.PipeReader" }
	M.PipeWriter = { __type = "io.PipeWriter" }

	function M.pipe()
		local wrTx, wrRx = sync.channel()
		local rdTx, rdRx = sync.channel()
		local doneTx, doneRx = sync.channel()
		local reader = { _wr_chan = wrRx, _rd_chan = rdTx, _done_chan = doneRx }
		setmetatable(reader, M.PipeReader)
		M.PipeReader.__index = M.PipeReader

		local writer = { _wr_chan = wrTx, _rd_chan = rdRx, _done_chan = doneTx }
		setmetatable(writer, M.PipeWriter)
		M.PipeWriter.__index = M.PipeWriter

		return writer, reader
	end

	function M.PipeReader:read(dst, size)
		if self._src == nil then
			select {
				[self._wr_chan] = function(s)
					self._src = s
				end,
				[self._done_chan] = function() end,
			}
		end
		if self._src == nil then error(M.errors.closed) end

		local ptr, len = self._src:ref()
		local read = math.min(len, size or 4096)
		dst:putcdata(ptr, read)
		self._src:skip(read)
		if read >= len then self._src = nil end
		self._rd_chan:send(read)

		return read
	end

	M.PipeReader.read_to_end = M.read_to_end

	function M.PipeWriter:write(buf)
		if self._done_chan:is_closed() then error(M.errors.closed) end

		local len = #buf
		if len == 0 then return 0 end

		local n = nil
		local ok = pcall(self._wr_chan.send, self._wr_chan, buf)
		if not ok then error(M.errors.closed) end

		while n ~= len do
			select {
				[self._rd_chan] = function(write)
					n = (n or 0) + write
				end,
				[self._done_chan] = function() end,
			}
		end

		if n == nil then error(M.errors.closed) end

		return n
	end
	M.PipeWriter.write_all = M.write_all

	function M.PipeWriter:close()
		if self._done_chan:is_closed() then error("pipe already closed") end
		self._done_chan:close()
		self._wr_chan:close()
	end
end
