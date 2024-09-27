local string = require("string")
local M = string

M.slice = M.sub
M.sub = nil

M.has_prefix = function(str, prefix)
	return string.slice(str, 0, #prefix) == prefix
end

M.has_suffix = function(str, suffix)
	return string.slice(str, -#suffix) == suffix
end

-- selene: allow(undefined_variable)
M.contains = __contains

return {
	__index = M,
}
