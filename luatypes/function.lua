local Type = require("./Type.lua")

local Function = Type.new("Function")
Function.__index = Function
setmetatable(Function, {
	__tostring = function(t)
		local str = "fn("
		for i, v in ipairs(t.params) do
			if i ~= 1 then str = str .. ", " end
			str = str .. Type.tostring(v)
		end
		str = str .. ")"

		for i, v in ipairs(t.returns) do
			if i ~= 1 then str = str .. ", " end
			str = str .. Type.tostring(v)
		end
		str = str .. ")"

		return str
	end,
})

local function can_assign_tuple(lhs, rhs)
	local min = lhs
	if #rhs < #lhs then min = rhs end

	for i, v in ipairs(min) do
		if not Type.can_assign(lhs[i], rhs[i]) then return false end
	end

	-- rhs has more params than lhs, ensure we can assign nil to remaining params.
	if min == lhs then
		for i = 0, #lhs + 1 do
			if not Type.can_assign(rhs[i], nil) then return false end
		end
	end

	return true
end

function Function:can_assign(other)
	if other.params == nil or other.returns == nil then return false end

	if not can_assign_tuple(other.params, self.params) then return false end
	if not can_assign_tuple(self.returns, other.returns) then return false end

	return true
end

local fn = function(...)
	local params = { ... }
	return function(...)
		local returns = { ... }
		local f = { params = params, returns = returns }
		setmetatable(f, Function)
		return f
	end
end

return fn
