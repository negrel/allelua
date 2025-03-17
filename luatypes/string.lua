local Type = require("./Type.lua")

local string = Type:new("string")
string.__index = string

function string:can_assign(other)
	return self == other or type(other.literal) == "string"
end

return string
