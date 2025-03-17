local Type = {}
Type.__index = Type
Type[Type] = true
setmetatable(Type, {
	__tostring = function(t)
		return t.kind
	end,
})

function Type.new(kind)
	local t = { kind = kind }
	setmetatable(t, Type)
	return t
end

local function _istype(t)
	return type(t) == "table" and t[Type] == true
end

function Type.tostring(n)
	if type(n) == "string" then return '"' .. tostring(n) .. '"' end
	return tostring(n)
end

function Type.can_assign(lhs, rhs)
	if _istype(lhs) then return lhs.can_assign(rhs) end
	return lhs == rhs -- literal
end

return Type
