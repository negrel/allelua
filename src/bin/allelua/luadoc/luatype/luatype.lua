local table = require("table")

local M = {}

M.IncompatibleTypeError = { __type = "IncompatibleTypeError" }

function M.IncompatibleTypeError:new(source, target, kind)
	local err = { source = source, target = target, kind = kind }
	setmetatable(err, self)
	self.__index = self
	return err
end

function M.IncompatibleTypeError:not_subtype(source, target)
	return self:new(source, target, "NotSubtype")
end

function M.IncompatibleTypeError:multiple(source, target, reasons)
	local err = self:new(source, target, "Multiple")
	err.reasons = reasons
	return err
end

function M.IncompatibleTypeError:field(source, target, field, reason)
	local err = self:new(source, target, "Field")
	err.field = field
	err.reasons = { reason }
	return err
end

function M.IncompatibleTypeError:__tostring(opts)
	opts.space = opts.space or 2
	opts.depth = opts.depth or 0

	local space = opts.space <= 0 and ""
		or "\n" .. string.rep(" ", opts.space * opts.depth)

	local inner_opts = {
		space = opts.space,
		depth = opts.depth + 1,
	}

	local result = space
		.. self.source.name
		.. " is not compatible with "
		.. self.target.name

	if self.kind == "NotSubtype" then
		result = space
			.. self.source.name
			.. " is not a subtype of "
			.. self.target.name
	elseif self.kind == "Field" then
		result = space
			.. 'field "'
			.. self.field
			.. '" of type '
			.. self.source.name
			.. " is not compatible with "
			.. self.target.name
	end

	if self.reasons and #self.reasons > 0 then result = result .. " because:" end
	for _, reason in ipairs(self.reasons or {}) do
		result = result .. tostring(reason, inner_opts)
	end

	return result
end

M.Type = { __type = "Type", name = "any" }

function M.Type:new(name, mt_type)
	local type = { name = name, metatable = mt_type }
	setmetatable(type, self)
	self.__index = self
	return type
end

function M.Type:constant(value)
	local name = tostring(value)
	local v_type = rawtype(value)

	-- quote constant if type is string.
	if v_type == "string" then name = '"' .. name .. '"' end

	local parent = M.Env:get_type_by_name(v_type)
	local type = parent:subtype(name)
	type.constant = true
	return type
end

function M.Type:subtype(name)
	local subtype = self:new(name)
	subtype.parent = self
	return subtype
end

-- is_assignable_to returns true if that type is assignable to target type.
function M.Type:is_assignable_to(target)
	return target:satisfy_requirements(self)
end

-- satisfy_requirements returns false along a reason if given type doesn't
-- satisfy requirements to be assignable to self.
-- This function is overrided by subclass of type.
function M.Type:satisfy_requirements(target)
	-- target is assignable to primitive if it has the same type...
	if target.name == self.name then return true end

	-- ...or is a subtype.
	if target.parent and target.parent:is_assignable_to(self) then return true end

	return false, M.IncompatibleTypeError:not_subtype(target, self)
end

function M.Type:get_field(_name)
	return M.Env:get_type_by_name("nil"), false
end

M.AbstractType = M.Type:new("AbstractType")
M.AbstractType.super = M.Type

function M.AbstractType:new(name, params, concretise)
	local type = self.super.new(self, name)
	type.params = params
	type.concretise = concretise or error("concretise parameter is missing")
	return type
end

M.InterfaceType = M.Type:new("InterfaceType")
M.InterfaceType.super = M.Type

function M.InterfaceType:new(name, ...)
	local type = M.InterfaceType.super.new(self, name)
	type.requirements = { ... }
	return type
end

-- satisfy_requirements returns false along a reason if given
-- type doesn't satisfy requirements of self.
function M.InterfaceType:satisfy_requirements(target)
	local reasons = {}

	for _, check in ipairs(self.requirements) do
		local ok, reason = check(self, target)
		if not ok then table.push(reasons, reason) end
	end

	if #reasons == 0 then return true end

	return false, M.IncompatibleTypeError:multiple(target, self, reasons)
end

M.UnionType = M.Type:new("UnionType")
M.UnionType.super = M.Type

function M.UnionType:new(...)
	local types = { ... }
	local types_name = table.map(clone(types), function(k, t)
		return t.name
	end)
	local name = table.concat(types_name, " | ")

	local type = M.UnionType.super.new(self, name)
	type.types = types
	type.union = true
	return type
end

function M.UnionType:is_assignable_to(target)
	-- All types must be assignable to assign to target.

	local reasons = {}

	for _, type in ipairs(self.types) do
		local ok, reason = type:is_assignable_to(target)
		if not ok then table.push(reasons, reason) end
	end

	if #reasons == 0 then return true end

	return false, M.IncompatibleTypeError:multiple(self, target, reasons)
end

function M.UnionType:satisfy_requirements(target)
	-- Target must satisfy requirements of a single variant to satisfy union.

	local reasons = {}

	for _, type in ipairs(self.types) do
		local ok, reason = target:is_assignable_to(type)
		if ok then return true end
		table.push(reasons, reason)
	end

	return false, M.IncompatibleTypeError:multiple(target, self, reasons)
end

M.IntersectionType = M.Type:new("IntersectionType")
M.IntersectionType.super = M.Type

function M.IntersectionType:new(...)
	local types = { ... }
	local name = table.concat(types, " + ")
	local type = M.IntersectionType.super.new(self, name)
	self.types = types
	return type
end

function M.IntersectionType:is_assignable_to(target)
	-- A single type must be assignable to assign to target.
	local reasons = {}

	for _, type in ipairs(self.types) do
		local ok, reason = type:is_assignable_to(target)
		if ok then return true end
		table.push(reasons, reason)
	end

	if #reasons == 0 then return true end

	return false, M.IncompatibleTypeError:multiple(target, self, reasons)
end

function M.IntersectionType:satisfy_requirements(target)
	-- Target must satisfy requirements of all variant to satisfy intersection.

	local reasons = {}

	for _, type in ipairs(self.types) do
		local ok, reason = target:is_assignable_to(type)
		if not ok then table.push(reasons, reason) end
	end

	if #reasons == 0 then return true end

	return false, M.IncompatibleTypeError:multiple(target, self, reasons)
end

M.StructType = M.InterfaceType:new("StructType")
M.StructType.super = M.InterfaceType

function M.StructType:new(name, fields)
	local requirements = {}
	for _, field in ipairs(fields) do
		table.push(requirements, function(struct, target)
			local target_type = target:get_field(field.name)
			local ok, reason = target_type:is_assignable_to(field.type)
			if not ok then
				return false,
					M.IncompatibleTypeError:field(
						target_type,
						field.type,
						field.name,
						reason
					)
			end
			return ok
		end)
	end

	local type = M.StructType.super.new(self, name, table.unpack(requirements))
	type.fields = fields
	return type
end

function M.StructType:get_field(name)
	for _, field in ipairs(self.fields) do
		if field.name == name then return field.type, true end
	end

	return M.Env:get_type_by_name("nil"), false
end

M.AliasType = M.Type:new("AliasType")
M.AliasType.super = M.Type

function M.AliasType:new(name, alias)
	local type = M.AliasType.super.new(self, name, alias.metatable)
	type.alias = alias
	setmetatable(type, {
		__index = type.alias,
	})
	return type
end

M.VariadicType = M.AbstractType:new(
	"VariadicType",
	{ "T" },
	function(_variadic, params)
		local type = M.AliasType:new(params.T.name, params.T)
		type.variadic = true
		return type
	end
)

function M.VariadicType:new(type)
	return self:concretise { T = type }
end

function M.VariadicType:satisfy_requirements(target)
	return self.alias:satisfy_requirements(target)
end

M.TupleType = M.InterfaceType:new("TupleType")
M.TupleType.super = M.Type

function M.TupleType:new(...)
	local types = { ... }
	local types_name = table.map(clone(types), function(i, t)
		return i, t.name
	end)
	local name = "(" .. table.concat(types_name, ", ") .. ")"
	local type = M.TupleType.super.new(self, name)
	type.items = types

	for i, t in ipairs(types) do
		if i ~= #types and t.variadic == true then
			error("variadic type must be the last")
		end
	end

	return type
end

function M.TupleType:satisfy_requirements(target)
	for i, type in ipairs(self.items) do
		-- check remaining args
		if type.variadic then
			local j = i
			while true do
				local target_type, ok = target:get_field(j)
				if not ok then return true end

				local ok, reason = target_type:is_assignable_to(type)
				if not ok then
					return false,
						M.IncompatibleTypeError:field(target, self, tostring(i), reason)
				end

				j = j + 1
			end
		else
			local target_type = target:get_field(i)
			local ok, reason = target_type:is_assignable_to(type)
			if not ok then
				return false,
					M.IncompatibleTypeError:field(target, self, tostring(i), reason)
			end
		end
	end

	return true
end

function M.TupleType:get_field(nth)
	if self.items[nth] then return self.items[nth], true end
	return M.Env:get_type_by_name("nil"), false
end

M.FunctionType = M.AbstractType:new(
	"FunctionType",
	{ "P", "R" },
	function(_function, params)
		local p = params.P
		local r = params.R
		return M.StructType:new(
			p.name .. " => " .. r.name,
			{ { name = "params", type = p }, { name = "returns", type = r } }
		)
	end
)
M.FunctionType.super = M.AbstractType

function M.FunctionType:new(params, returns)
	return self:concretise { P = params, R = returns }
end

--- Env define a typing environment.
--- @see https://mukulrathi.com/create-your-own-programming-language/intro-to-type-checking/#typing-environments
M.Env = {
	_types = {
		["nil"] = M.Type:new("nil"),
		boolean = M.Type:new("boolean"),
		string = M.Type:new("string"),
		number = M.Type:new("number"),

		any = M.InterfaceType:new("any"),

		-- interfaces types
		Metatable = M.AbstractType:new(
			"Metatable",
			{ "T" },
			function(_self, params)
				if params.T.metatable then return params.T.metatable end

				return M.Env.get_type_by_name("nil")
			end
		),
		KeysOf = M.AbstractType:new("KeysOf", { "T" }, function(_self, params)
			local keys = table.map(params.T.fields, function(i, field)
				return i, M.Type:constant(field.name)
			end)
			return M.UnionType:new(table.unpack(keys))
		end),
		Pick = M.AbstractType:new("Pick", { "T" }, function(_self, params)
			local struct = params.T
			local pick = params[1]
			local picked_fields = {}

			for _, field in ipairs(struct.fields or {}) do
				local key_type = M.Type:constant(field.name)
				if key_type:is_assignable_to(pick) then
					table.push(picked_fields, field)
				end
			end

			return M.StructType:new(
				"Pick<" .. struct.name .. ", " .. pick.name .. ">",
				picked_fields
			)
		end),
	},
}

--- register_type adds a type to environment.
function M.Env:register_type(type)
	self._types[type.name] = type
end

--- get_type_by_name returns type with given name in the environment.
function M.Env:get_type_by_name(name)
	local type = self._types[name]

	--- if not found, check in parent env.
	if not type and self.parent then return self.parent:get_type_by_name(name) end

	return type
end

--- new_child creates a new child environment with `self` as parent.
function M.Env:new_child()
	local env = { _parent = self }
	setmetatable(env, self)
	self.__index = self

	return env
end

M.Env:register_type(M.Env:get_type_by_name("number"):subtype("integer"))

M.Env:get_type_by_name("boolean").metatable =
	M.StructType:new("Metatable<boolean>", {
		{
			name = "__eq",
			type = M.FunctionType:new(
				M.Env:get_type_by_name("any"),
				M.Env:get_type_by_name("any")
			),
		},
	})

local types = M.Env._types

local function set_type_metatable(type_name, meta)
	local t = types[type_name]
	local fields = {}

	for name, type in pairs(meta) do
		table.push(fields, {
			name = name,
			type = M.FunctionType:new(
				M.TupleType:new(table.unpack(type.params)),
				M.TupleType:new(table.unpack(type.returns))
			),
		})
	end

	t.metatable = M.StructType:new("Metatable<" .. type_name .. ">", fields)
end

set_type_metatable("boolean", {
	__eq = {
		params = { types.boolean, types.boolean },
		returns = { types.boolean },
	},
	__tostring = {
		params = { types.boolean },
		returns = { types.string },
	},
})

set_type_metatable("string", {
	__eq = {
		params = { types.string, types.string },
		returns = { types.boolean },
	},
	__tostring = {
		params = { types.string },
		returns = { types.string },
	},
})

set_type_metatable("number", {
	__eq = {
		params = { types.number, types.number },
		returns = { types.boolean },
	},
	__lt = {
		params = { types.number, types.number },
		returns = { types.boolean },
	},
	__le = {
		params = { types.number, types.number },
		returns = { types.boolean },
	},
	__add = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__sub = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__mul = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__div = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__unm = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__mod = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__pow = {
		params = { types.number, types.number },
		returns = { types.number },
	},
	__tostring = {
		params = { types.number },
		returns = { types.string },
	},
})

return M
