return function(M)
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
			reader:close()
			writer:close()
		end

		return total
	end
end
