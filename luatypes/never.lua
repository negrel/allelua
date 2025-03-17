local Type = require("./Type.lua")

local never = Type:new("never")
never.__index = never

function never:can_assign(_other)
	return false
end

return never
