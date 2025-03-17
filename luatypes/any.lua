local Type = require("./Type.lua")

local any = Type:new("any")
any.__index = any

function any:can_assign(_other)
	return true
end

return any
