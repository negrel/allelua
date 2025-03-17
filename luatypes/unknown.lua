local Type = require("./Type.lua")

local unknown = Type:new("unknown")
unknown.__index = unknown

function unknown:can_assign(_other)
	return true
end

return unknown
