string.slice = string.sub
string.sub = nil

string.replace_all = string.gsub
string.gsub = nil

function string.has_prefix(str, prefix)
	return string.slice(str, 1, #prefix) == prefix
end

function string.has_suffix(str, suffix)
	return string.slice(str, -#suffix) == suffix
end

function string.strip_prefix(str, prefix)
	if string.has_prefix(str, prefix) then
		return string.slice(str, #prefix + 1)
	end
	return str
end

function string.strip_suffix(str, suffix)
	if string.has_suffix(str, suffix) then
		return string.slice(str, 1, -#suffix - 1)
	end
	return str
end
