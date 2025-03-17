local Type = require("./Type.lua")

local Union = Type.new("union")
Union.__index = Union
setmetatable(Union, {
	__tostring = function(t)
		local str = "union("
		for i, v in ipairs(t.variants) do
			if i ~= 1 then str = str .. ", " end
			str = str .. Type.tostring(v)
		end
		str = str .. ")"

		return str
	end,
})

function Union:can_assign(other)
	-- Special case: union.
	if other.name == "union" then
		for _, v in ipairs(other.variants) do
			if not self:can_assign(other) then return false end
		end
		return true
	end

	for _, v in ipairs(self.variants) do
		if Type.can_assign(v, other) then return true end
	end

	return false
end

local union = function(...)
	local u = { variants = { ... } }
	setmetatable(u, Union)
	return u
end

return union
