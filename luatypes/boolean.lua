local Type = require("./Type.lua")

local boolean = Type:new("boolean")
boolean.__index = boolean

function boolean:can_assign(other)
	return self == other or type(other) == "boolean"
end

return boolean
