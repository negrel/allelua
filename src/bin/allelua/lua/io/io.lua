return function(M)
	local math = require("math")
	local sync = require("sync")
	local buffer = require("string").buffer

	M.copy = function(reader, writer, opts)
		opts = opts or {}
		opts.flush = opts.flush or false
		opts.close = opts.close or false

		local total = 0

		if rawtype(reader.write_to) == "function" then
			while true do
				local ok, err_or_write = pcall(reader.write_to, reader, writer)
				if not ok then
					if err_or_write.kind == "Closed" and total ~= 0 then break end
					error(err_or_write)
				end

				if opts.flush then writer:flush() end
				if err_or_write == 0 then break end
				total = total + err_or_write
			end
		else
			local buf = buffer.new()
			while true do
				-- Read into buffer.
				local ok, err_or_read = pcall(reader.read, reader, buf, 4096)
				if not ok then
					if err_or_read.kind == "BrokenPipe" then break end
					error(err_or_read)
				end
				if err_or_read == 0 then break end

				-- Write from buffer.
				local ok, err_or_write = pcall(writer.write_all, writer, buf)
				if not ok then
					if err_or_write.kind == "Closed" and total ~= 0 then break end
					error(err_or_write)
				end

				if opts.flush then writer:flush() end
				total = total + err_or_write
			end
		end

		if opts.close then
			if rawtype(reader.close) == "function" then reader:close() end
			if rawtype(writer.close) == "function" then writer:close() end
		end

		return total
	end

	function M.read_to_end(self)
		local buf = buffer.new()
		while true do
			local ok, err = pcall(self.read, self, buf)
			if not ok then
				if type(err) == "IoError" and err.kind == "Closed" then
					return buf:tostring()
				end
				error(err)
			end
		end
	end

	M.PipeReader = { __type = "PipeReader" }
	M.PipeWriter = { __type = "PipeWriter" }

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
		if self._src == nil then error(M.ClosedError) end

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
		if self._done_chan:is_closed() then error(M.ClosedError) end

		local len = #buf
		if len == 0 then return 0 end

		local n = nil
		local ok = pcall(self._wr_chan.send, self._wr_chan, buf)
		if not ok then error(M.ClosedError) end

		while n ~= len do
			select {
				[self._rd_chan] = function(write)
					n = (n or 0) + write
				end,
				[self._done_chan] = function() end,
			}
		end

		if n == nil then error(M.ClosedError) end

		return n
	end

	function M.PipeWriter:close()
		if self._done_chan:is_closed() then error("pipe already closed") end
		self._done_chan:close()
		self._wr_chan:close()
	end
end
