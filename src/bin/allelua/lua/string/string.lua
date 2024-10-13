return function(Regex, extra)
	local buffer = require("string.buffer")
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

	for k, v in pairs(extra) do
		M[k] = v
	end

	return {
		__index = M,
	}
end
