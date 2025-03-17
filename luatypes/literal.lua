local Type = require("./Type.lua")

local literal = Type:new("literal")
literal.__index = literal
setmetatable(literal, {
	__tostring = function(t)
		return t.literal
	end,
})

function literal.new(v)
	local l = { literal = v }
	setmetatable(l, literal)
	return l
end

function literal:can_assign(other)
	return self.literal == other.literal
end

return literal
