return function(Regex, extra)
	local buffer = require("string.buffer")
	local io = require("io")
	local math = require("math")
	local string = require("string")
	local M = string

	M.buffer = buffer
	M.Regex = Regex

	-- Rename sub to slice.
	M.slice = M.sub
	M.sub = nil

	-- Remove Lua regex functions.
	M.gmatch = nil
	M.gsub = nil
	M.match = nil

	M.has_prefix = function(str, prefix)
		return string.slice(str, 0, #prefix) == prefix
	end

	M.has_suffix = function(str, suffix)
		return string.slice(str, -#suffix) == suffix
	end

	M.toregex = function(str, escaped)
		if escaped then
			return Regex.new(Regex.escape(str))
		else
			return Regex.new(str)
		end
	end

	local function regex_or_escaped_regex(str)
		if type(str) == "Regex" then
			return str
		else
			return Regex.new(Regex.escape(str))
		end
	end

	M.find_iter = function(str, pattern, find_start)
		find_start = find_start or 0
		local re = regex_or_escaped_regex(pattern)

		return function(str)
			local substr, i, j = str:find(re, find_start)
			find_start = (j or 0)
			return substr, i, j
		end,
			str
	end

	M.captures_iter = function(str, pattern, captures_start)
		captures_start = captures_start or 0
		local re = regex_or_escaped_regex(pattern)
		return function(str)
			local captures = str:captures(re, captures_start)
			if captures and #captures > 0 then
				captures_start = captures[#captures]["end"]
			end
			return captures
		end,
			str
	end

	M.contains = function(str, pattern)
		return M.find(str, pattern) ~= nil
	end

	M.match = function(str, pattern)
		return regex_or_escaped_regex(pattern):is_match(str)
	end

	M.quote = function(str)
		return ("%q"):format(str)
	end

	for k, v in pairs(extra) do
		M[k] = v
	end

	-- Buffer is a wrapper around string.buffer that implements io.Reader,
	-- io.Writer, io.ReaderFrom and io.WriterTo.
	M.Buffer = { __type = "string.Buffer" }
	M.Buffer.__index = M.Buffer

	function M.Buffer.new(...)
		local buf = { inner = buffer.new(...) }
		setmetatable(buf, M.Buffer)
		return buf
	end

	function M.Buffer:reset()
		self.inner:reset()
	end

	function M.Buffer:free()
		self.inner:free()
	end

	function M.Buffer:read(buf)
		local _, available = buf:reserve(0)
		local ptr, len = self.inner:ref()
		local read = math.min(available, len)
		buf:putcdata(ptr, read)
		self.inner:skip(read)
		return read
	end
	function M.Buffer:read_to_end()
		return self.inner:get()
	end

	function M.Buffer:write_to(writer)
		io.write_all(writer, self.inner)
		self:skip(#self)
	end

	function M.Buffer:write(buf)
		self.inner:put(buf)
		return #buf
	end
	M.Buffer.write_string = io.write_string
	M.Buffer.write_all = io.write_all

	function M.Buffer:read_from(reader)
		local chunk_size = math.max(self:available(), 4096)

		while true do
			self.inner:reserve(chunk_size)
			local ok, read = pcall(reader.read, reader, self.inner)
			if not ok then
				local err = read
				if err:is(io.errors.closed) then break end
				error(err)
			end
			if read == 0 then break end
		end
	end

	function M.Buffer:available()
		local _, len = self.inner:reserve(0)
		return len
	end

	function M.Buffer:reserve(n)
		self.inner:reserve(n)
	end

	function M.Buffer:skip(n)
		self.inner:skip(n)
	end

	function M.Buffer:__len()
		return #self.inner
	end

	function M.Buffer:tostring()
		return self.inner:tostring()
	end
	M.Buffer.__tostring = M.Buffer.tostring

	return {
		__index = M,
	}
end
