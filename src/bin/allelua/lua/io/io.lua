return function(M)
	local ffi = require("ffi")
	local math = require("math")
	local sync = require("sync")
	local error = require("error")
	local libbuf = require("string.buffer")

	libbuf.copy = function(src, dst, len)
		local dst_ptr, dst_len = dst:reserve(len or 0)
		local src_len = #src
		if rawtype(src) == "buffer" then src = src:ref() end
		local copied = math.min(dst_len, src_len, len or dst_len)
		ffi.copy(dst_ptr, src, copied)
		return copied
	end

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
			local buf = libbuf.new()
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
		local buf = libbuf.new(buf_size)

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
				local clone = libbuf.new()
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
		local buf = libbuf.new(#str)
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

	M.discard = {
		write = function(_, _buf) end,
	}

	M.BufReader = { __type = "io.BufReader" }
	M.BufReader.__index = M.BufReader

	function M.BufReader.new(reader, opts)
		assert(reader, "reader must not be nil")

		opts = opts or {}
		local cap = opts.size or M.default_buffer_size
		assert(cap > 0, "buffer size must be greater than 0")
		local buf = libbuf.new(cap)
		local buf_reader = { reader = reader, buffer = buf, _cap = cap }
		setmetatable(buf_reader, M.BufReader)
		return buf_reader
	end

	function M.BufReader:buffered()
		return #self.buffer
	end

	function M.BufReader:available()
		return self._cap - #self.buffer
	end

	function M.BufReader:discard(n)
		assert(n > 0, "discard count must be a positive number")

		-- Discard from buffer if possible.
		local buffered = self:buffered()
		if buffered <= n then
			-- Reset entire buffer if possible.

			self.buffer:reset()
			n = n - buffered
			buffered = 0
		elseif n < buffered then
			-- Otherwise just skip data in buffer.
			self.buffer:skip(n)
		end

		-- Finally, discard from reader.
		while n > 0 do
			local read = self.reader:read(self.buffer)

			-- We read more than we need to discard.
			if read > n then
				self.buffer:commit(read)
				self.buffer:skip(n)
			end
			n = n - read
		end
	end

	function M.BufReader:read(buf)
		local copied = 0

		-- Copy data if available.
		if #self.buffer > 0 then
			copied = libbuf.copy(self.buffer, buf)
			buf:commit(copied)
			self.buffer:skip(copied)
			-- There still is data in internal buffer, this means given buffer
			-- is full.
			if #self.buffer ~= 0 then return copied end
		end
		-- No data in internal buffer, reset it to reuse memory.
		self.buffer:reset()

		-- If given buffer has more space than internal buffer, read directly into
		-- it.
		local _, available = buf:reserve(0)
		if available > self._cap then return self.reader:read(buf) end

		-- Read in internal buffer.
		local read = self.reader:read(self.buffer)
		if read == 0 then return 0 end

		-- Copy part of buffered data into given buffer.
		local copied2 = libbuf.copy(self.buffer, buf)
		buf:commit(copied2)
		self.buffer:skip(copied2)
		return copied + copied2
	end

	M.BufWriter = { __type = "io.BufWriter" }
	M.BufWriter.__index = M.BufWriter

	function M.BufWriter:__len()
		return self._cap
	end

	function M.BufWriter.new(writer, opts)
		assert(writer, "writer must not be nil")

		opts = opts or {}
		local cap = opts.size or M.default_buffer_size
		assert(cap > 0, "buffer size must be greater than 0")
		local buf = libbuf.new(cap)
		local buf_writer = { writer = writer, buffer = buf, _cap = cap }
		setmetatable(buf_writer, M.BufWriter)
		return buf_writer
	end

	function M.BufWriter:buffered()
		return #self.buffer
	end

	function M.BufWriter:available()
		return self._cap - #self.buffer
	end

	function M.BufWriter:write(buf)
		-- No more space in internal buffer.
		if #buf > self:available() then self:flush() end

		-- Given buffer doesn't fit into internal buffer, pass it directly to writer
		-- to avoid copy.
		if #buf > self._cap then return self.writer:write(buf) end

		-- Space available, simply copy given buffer.
		local copied = libbuf.copy(buf, self.buffer)
		self.buffer:commit(copied)
		return copied
	end

	function M.BufWriter:flush()
		while #self.buffer > 0 do
			local write = self.writer:write(self.buffer)
			self.buffer:skip(write)
		end
		self.buffer:reset()
	end
end
