local Type = require("./Type.lua")

local number = Type:new("number")
number.__index = number

function number:can_assign(other)
	return self == other or type(other.literal) == "number"
end

return number
