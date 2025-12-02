string.slice = string.sub
string.sub = nil

string.replace_all = string.gsub
string.gsub = nil

string.Buffer = require("string.buffer")
local buffer_mt = debug.getmetatable(string.Buffer.new())
buffer_mt.to_string = buffer_mt.tostring
buffer_mt.tostring = nil

--- Reports whether the string str begins with prefix.
function string.has_prefix(str, prefix)
	return string.slice(str, 1, #prefix) == prefix
end

--- Reports whether the string str ends with suffix.
function string.has_suffix(str, suffix)
	return string.slice(str, -#suffix) == suffix
end

--- Returns string str withtout the provided leading prefix string. If str
--- doesn't start with prefix, str is returned unchanged.
function string.strip_prefix(str, prefix)
	if string.has_prefix(str, prefix) then
		return string.slice(str, #prefix + 1)
	end
	return str
end

--- Returns string str withtout the provided trailing suffix string. If str
--- doesn't end with prefix, str is returned unchanged.
function string.strip_suffix(str, suffix)
	if string.has_suffix(str, suffix) then
		return string.slice(str, 1, -#suffix - 1)
	end
	return str
end
