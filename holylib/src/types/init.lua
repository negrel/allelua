local M = {}

local Type = {}
Type.__index = Type

--- _is_type returns true if self is of type Type or a subtype of it.
function M._is_type(t)
	local mt = getmetatable(t)
	return mt and mt.is_type or false
end

--- Type constructor.
local function new_type(string)
	local t = { string = string, is_type = true }
	return t
end

--- to_type_string returns name of type v if v is a Type. Otherwise it returns
--- a stringified version of v.
local function to_type_string(v)
	if not M._is_type(v) then
		-- if type(v) == "string" then
		-- 	return '"' .. v .. '"'
		-- end
		-- return tostring(v)
		return type(v)
	end

	local mt = getmetatable(v)
	if type(mt.string) == "string" then
		return mt.string
	end

	return mt.string(v)
end
M.to_type_string = to_type_string

local function not_assignable(lhs, rhs)
	error("type " .. to_type_string(rhs) .. " is not assignable to type " .. to_type_string(lhs))
end

function M._assign(lhs, rhs)
	if M._is_type(lhs) then
		local mt = getmetatable(lhs)
		local _, err = pcall(mt.__assign, lhs, rhs)
		return err
	end

	-- Literals.
	if lhs == rhs then return end

	-- Default to error.
	local _, err = pcall(not_assignable, lhs, rhs)
	return err
end

function Type:__tostring()
	error("__tostring operation not supported by type " .. to_type_string(self))
end

-- __add returns result Type of an addition between two types.
function Type:__add()
	error("arithmetic operation + not supported by type " .. to_type_string(self))
end

-- __unm returns result Type of a unary minus.
function Type:__unm()
	error("arithmetic operation unary - not supported by type " .. to_type_string(self))
end

-- __sub returns result Type of a substraction between two types.
function Type:__sub()
	error("arithmetic operation - not supported by type " .. to_type_string(self))
end

-- __mod returns result Type of a module between two types.
function Type:__mod()
	error("arithmetic operation % not supported by type " .. to_type_string(self))
end

-- __div returns result Type of a division between two types.
function Type:__div()
	error("arithmetic operation / not supported by type " .. to_type_string(self))
end

-- __mul returns result Type of a multiplication between two types.
function Type:__mul()
	error("arithmetic operation * not supported by type " .. to_type_string(self))
end

-- __pow returns result Type of a power between two types.
function Type:__pow()
	error("arithmetic operation ˆ not supported by type " .. to_type_string(self))
end

-- __eq returns result Type of an equal comparison between two types.
function Type:__eq()
	error("operation == not supported by type " .. to_type_string(self))
end

-- __le returns result Type of a less or equal comparison between two types.
function Type:__le()
	error("operation <= not supported by type " .. to_type_string(self))
end

-- __lt returns result Type of a less than comparison between two types.
function Type:__lt()
	error("operation < not supported by type " .. to_type_string(self))
end

-- __concat returns result Type of a concatenation between two types.
function Type:__concat()
	error("operation .. not supported by type " .. to_type_string(self))
end

-- __len returns result Type of length operator.
function Type:__len()
	error("operation # not supported by type " .. to_type_string(self))
end

-- __index returns result Type of index.
function Type:__index()
	error("index not supported on type " .. to_type_string(self))
end

-- __call returns result Type of call operation.
function Type:__call()
	error("function call not supported on type " .. to_type_string(self))
end

-- boolean primitive type.
local boolean = new_type("boolean")
M.boolean = setmetatable({}, boolean)

function boolean:__assign(rhs)
	if self ~= rhs and type(rhs) ~= "boolean" then
		not_assignable(self, rhs)
	end
end

function boolean:__eq(rhs)
	return boolean
end

-- string primitive type.
local string = new_type("string")
M.string = setmetatable({}, string)

function string:__assign(rhs)
	if self ~= rhs and type(rhs) ~= "string" then
		not_assignable(self, rhs)
	end
end

function string.__concat(lhs, rhs)
	if to_type_string(lhs) == "string" and to_type_string(rhs) == "string" then
		return string
	end
	error("operation .. not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function string:__index(_rhs)
	return nil
end

function string.__eq(lhs, rhs)
	if to_type_string(lhs) == "string" and to_type_string(rhs) == "string" then
		return boolean
	end
	error("comparison not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

string.__lt = string.__eq
string.__le = string.__eq

local number = new_type("number")
M.number = setmetatable({}, number)

function number:__assign(rhs)
	if to_type_string(rhs) ~= "number" then
		not_assignable(self, rhs)
	end
end

function number:__unm()
	return number
end


function number.__add(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return number
	end

	error("operation + not supported for types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__sub(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return number
	end
	error("operation - not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__mod(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return number
	end
	error("operation % not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__div(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return number
	end
	error("operation / not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__mul(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return number
	end
	error("operation * not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__pow(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return number
	end
	error("operation ˆ not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__eq(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return boolean
	end
	error("operation == not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__le(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return boolean
	end
	error("operation <= not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

function number.__lt(lhs, rhs)
	if to_type_string(lhs) == "number" and to_type_string(rhs) == "number" then
		return boolean
	end
	error("operation < not supported with types " .. to_type_string(lhs) .. " and " .. to_type_string(rhs))
end

local func = new_type("function")
M.func = func
func.__index = func

local function assign_tuple(lhs, rhs)
	local min = lhs
	if #rhs < #lhs then min = rhs end

	for i, v in ipairs(min) do
		M.assign(lhs[i], rhs[i])
	end

	-- lhs has more values than rhs, ensure we can assign nil to extra values.
	if min == rhs then
		for i = 0, #min + 1 do
			M.assign(lhs[i], nil)
		end
	end
end

function func.__assign(lhs, rhs)
	if self == rhs then return true end
	if not M._is_type(lhs) or M._is_type(rhs) then
		not_assignable(lhs, rhs)
	end

	lhs = getmetatable(lhs)
	rhs = getmetatable(rhs)

	if not assign_tuple(rhs.params or {}, lhs.params or {}) then
		return false
	end
	if not assign_tuple(lhs.results or {}, rhs.results or {}) then
		return false
	end
end

function func:string()
	local mt = getmetatable(self)
	local str = "fn("
	for i, v in ipairs(mt.params) do
		if i > 1 then str = str .. ", " end
		str = str .. to_type_string(v)
	end
	str = str .. ") ("
	for i, v in ipairs(mt.results) do
		if i > 1 then str = str .. ", " end
		str = str .. to_type_string(v)
	end
	return str .. ")"
end

--- function type constructor.
function M.fn(...)
	local params = { ... }
	return function(...)
		local results = { ... }
		local mt = setmetatable({
			params = params,
			results = results,
		}, func)
		return setmetatable({}, mt)
	end
end

-- We store variable types in scope tables.
local scopes = {}
M.scopes = scopes

local function new_scope(parent)
	return setmetatable({}, { __index = parent })
end

scopes.root = new_scope(nil)
scopes.current = scopes.root

function scopes.push()
	scopes.current = new_scope(scopes.current)
	return scopes.current
end

function scopes.pop()
	scopes.current = getmetatable(scopes.current).__index
	if scopes.current == nil then error("root scope removed") end
	return scopes.current
end

--- Evaluates provided lua code in curret scope.
function M.eval_in_scope(expr)
	local f, err = loadstring(expr, "eval_in_scope")
	if err then error(err) end

	setfenv(f, scopes.current)

	local t = f()
	if M._is_type() then
		return t
	end

	return M[to_type_string(t)]
end

function M.eval_type(expr)
	local f, err = loadstring(expr, "eval_type")
	if err then error(err) end

	setfenv(f, M)

	local t = f()
	if M._is_type(t) then
		return t
	end

	return M[to_type_string(t)]
end

return M
