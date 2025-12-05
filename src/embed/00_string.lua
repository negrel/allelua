string.slice = string.sub
string.sub = nil

string.replace_all = string.gsub
string.gsub = nil

-- Set metatable of all strings to string module.
debug.setmetatable("", string)
string.__index = string

string.Buffer = require("string.buffer")
local buffer_mt = debug.getmetatable(string.Buffer.new())
buffer_mt.to_string = buffer_mt.tostring
buffer_mt.tostring = nil
buffer_mt.__metatable = "string.Buffer"

--- Reports whether the string `str` begins with prefix.
function string.has_prefix(str, prefix)
	return string.slice(str, 1, #prefix) == prefix
end

--- Reports whether the string `str` ends with suffix.
function string.has_suffix(str, suffix)
	return string.slice(str, -#suffix) == suffix
end

--- Returns string `str` withtout the provided leading prefix string. If `str`
--- doesn't start with prefix, `str` is returned unchanged.
function string.strip_prefix(str, prefix)
	if string.has_prefix(str, prefix) then
		return string.slice(str, #prefix + 1)
	end
	return str
end

--- Returns string `str` withtout the provided trailing suffix string. If `str`
--- doesn't end with prefix, `str` is returned unchanged.
function string.strip_suffix(str, suffix)
	if string.has_suffix(str, suffix) then
		return string.slice(str, 1, -#suffix - 1)
	end
	return str
end

--- Returns an iterator over lines of `str` without "\r\n" or "\n" suffix.
function string.lines(str)
	return string.gmatch(str, "([^\n]+)\r?\n")
end

--- Returns an iterator over whitespace separated words of `str`.
function string.words(str)
	return string.gmatch(str, "%s?(%S+)%s?")
end
