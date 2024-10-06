local luadoc = require("./luadoc.lua")
local t = require("test")

function assert_assignable(type, target)
	local ok, reason = type:is_assignable_to(target)
	assert(
		ok,
		"type "
			.. type.name
			.. " should be assignable to "
			.. target.name
			.. " but it isn't: "
			.. tostring(reason)
	)
end

local function table_length(tab)
	if rawtype(tab) == "table" then return #tab end
	return 0
end

function assert_incompatible_type_error_kind(actual, expected)
	if actual.kind ~= expected.kind then return false, actual end
	if expected.field and actual.field ~= expected.field then
		return false, actual
	end
	if expected.source and actual.source.name ~= expected.source then
		return false, actual
	end
	if expected.target and actual.target.name ~= expected.target then
		return false, actual
	end

	for i, expected in ipairs(expected.reasons or {}) do
		local actual = actual.reasons[i]
		local ok, reason = assert_incompatible_type_error_kind(actual, expected)
		if not ok then return false, reason end
	end

	if table_length(actual.reasons) ~= table_length(expected.reasons) then
		return false, actual
	end

	return true
end

function assert_not_assignable(type, target, expected_error)
	local ok, reason = type:is_assignable_to(target)
	assert(
		not ok,
		"type "
			.. type.name
			.. " should NOT be assignable to "
			.. target.name
			.. " but it is."
	)

	if expected_error then
		-- selene: allow(shadowing)
		local ok, reason =
			assert_incompatible_type_error_kind(reason, expected_error)
		if not ok then
			error(
				"type "
					.. type.name
					.. " is NOT assignable to "
					.. target.name
					.. " but for the wrong reason: "
					.. tostring(reason)
					.. "\nexpected reason: "
					.. tostring(expected_error)
			)
		end
	end
end

t.test(
	"integer, subtype of number, is assignable to number but NOT vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local integer = luadoc.Env:get_type_by_name("integer")

		assert_assignable(integer, number)
		assert_not_assignable(number, integer, { kind = "NotSubtype" })
	end
)

t.test("boolean is NOT assignable to number", function()
	local number = luadoc.Env:get_type_by_name("number")
	local boolean = luadoc.Env:get_type_by_name("boolean")

	assert_not_assignable(boolean, number, { kind = "NotSubtype" })
end)

t.test("constant true is assignable to boolean but NOT vice versa", function()
	local boolean = luadoc.Env:get_type_by_name("boolean")
	local ctrue = luadoc.Type:constant(true)

	assert_assignable(ctrue, boolean)
	assert_not_assignable(boolean, ctrue, { kind = "NotSubtype" })
end)

t.test(
	"string | integer is assignable to string | number but NOT vice versa",
	function()
		local string = luadoc.Env:get_type_by_name("string")
		local number = luadoc.Env:get_type_by_name("number")
		local integer = luadoc.Env:get_type_by_name("integer")

		local string_or_number = luadoc.UnionType:new(string, number)
		local string_or_integer = luadoc.UnionType:new(string, integer)

		assert_assignable(string_or_integer, string_or_number)
		assert_not_assignable(string_or_number, string_or_integer, {
			kind = "Multiple",
			source = "string | number",
			target = "string | integer",
			reasons = {
				{
					kind = "Multiple",
					source = "number",
					target = "string | integer",
					reasons = {
						{
							kind = "NotSubtype",
							source = "number",
							target = "string",
						},
						{
							kind = "NotSubtype",
							source = "number",
							target = "integer",
						},
					},
				},
			},
		})
	end
)

t.test(
	"{ x = number, y = number, z = number } is assignable to { x = number, y = number } but NOT vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local Vec2 = luadoc.StructType:new(
			"Vec2",
			{ { name = "x", type = number }, { name = "y", type = number } }
		)
		local Vec3 = luadoc.StructType:new("Vec3", {
			{ name = "x", type = number },
			{ name = "y", type = number },
			{ name = "z", type = number },
		})

		assert_assignable(Vec3, Vec2)
		assert_not_assignable(Vec2, Vec3, {
			kind = "Multiple",
			reasons = {
				{ kind = "Field", field = "z", reasons = { { kind = "NotSubtype" } } },
			},
		})
	end
)

t.test(
	'KeysOf<{x = number, y = number}> is assignable to "x" | "y" | "z" but NOT vice versa',
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local Vec2 = luadoc.StructType:new(
			"Vec2",
			{ { name = "x", type = number }, { name = "y", type = number } }
		)
		local KeysOf = luadoc.Env:get_type_by_name("KeysOf")

		local KeysOfVec2 = KeysOf:concretise { T = Vec2 }
		local x_or_y_or_z = luadoc.UnionType:new(
			luadoc.Type:constant("x"),
			luadoc.Type:constant("y"),
			luadoc.Type:constant("z")
		)

		assert_assignable(KeysOfVec2, x_or_y_or_z)
		assert_not_assignable(x_or_y_or_z, KeysOfVec2, {
			kind = "Multiple",
			source = '"x" | "y" | "z"',
			target = '"x" | "y"',
			reasons = {
				{
					kind = "Multiple",
					source = '"z"',
					target = '"x" | "y"',
					reasons = {
						{
							kind = "NotSubtype",
							source = '"z"',
							target = '"x"',
						},
						{
							kind = "NotSubtype",
							source = '"z"',
							target = '"y"',
						},
					},
				},
			},
		})
	end
)

t.test(
	'Pick<{ x = number, y = number }, "x"> is assignable to { x = number } and vice versa',
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local Vec1 =
			luadoc.StructType:new("Vec1", { { name = "x", type = number } })
		local Vec2 = luadoc.StructType:new(
			"Vec2",
			{ { name = "x", type = number }, { name = "y", type = number } }
		)
		local Pick = luadoc.Env:get_type_by_name("Pick")
		local PickX = Pick:concretise { T = Vec2, luadoc.Type:constant("x") }

		assert_assignable(PickX, Vec1)
		assert_assignable(Vec1, PickX)
	end
)

t.test(
	"(number, number) => { a = number, b = number, sum = number } is assignable to (number, number) => { a = number, b = number } but NOT vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local SumParams = luadoc.TupleType:new(number, number)
		local SumResult = luadoc.StructType:new("SumResult", {
			{ name = "a", type = number },
			{ name = "b", type = number },
			{ name = "sum", type = number },
		})
		local OtherResult = luadoc.StructType:new("OtherResult", {
			{ name = "a", type = number },
			{ name = "b", type = number },
		})

		local SumFunction = luadoc.FunctionType:new(SumParams, SumResult)
		local OtherFunction = luadoc.FunctionType:new(SumParams, OtherResult)

		assert_assignable(SumFunction, OtherFunction)
		assert_not_assignable(OtherFunction, SumFunction)
	end
)

t.test(
	"(number, number) => (number) is assignable to (number, number) => () but NOT vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local SumParams = luadoc.TupleType:new(number, number)
		local SumResult = luadoc.TupleType:new(number)
		local OtherResult = luadoc.TupleType:new()

		local SumFunction = luadoc.FunctionType:new(SumParams, SumResult)
		local OtherFunction = luadoc.FunctionType:new(SumParams, OtherResult)

		assert_assignable(SumFunction, OtherFunction)
		assert_not_assignable(OtherFunction, SumFunction)
	end
)

t.test(
	"(number, number) => (number) is assignable to (...number) => (number) but NOT vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local VariadicNumbers =
			luadoc.TupleType:new(luadoc.VariadicType:new(number))
		local PairParams = luadoc.TupleType:new(number, number)
		local SumResult = luadoc.TupleType:new(number)

		local PairFunction = luadoc.FunctionType:new(PairParams, SumResult)
		local VariadicFunction = luadoc.FunctionType:new(VariadicNumbers, SumResult)

		assert_assignable(PairFunction, VariadicFunction)
		assert_not_assignable(VariadicFunction, PairFunction, {
			kind = "Multiple",
			reasons = {
				{
					kind = "Field",
					field = "params",
					reasons = {
						{
							kind = "Field",
							field = "2",
							reasons = {
								{ kind = "NotSubtype", source = "nil", target = "number" },
							},
						},
					},
				},
			},
		})
	end
)

t.test(
	"() => (number) is assignable to (...number) => (number) and vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local VariadicNumbers =
			luadoc.TupleType:new(luadoc.VariadicType:new(number))
		local EmptyParams = luadoc.TupleType:new()
		local SumResult = luadoc.TupleType:new(number)

		local EmptyFunction = luadoc.FunctionType:new(EmptyParams, SumResult)
		local VariadicFunction = luadoc.FunctionType:new(VariadicNumbers, SumResult)

		assert_assignable(EmptyFunction, VariadicFunction)
		assert_assignable(VariadicFunction, EmptyFunction)
	end
)

t.test(
	"Vec2 alias of { x: number, y: number } is assignable to { x: number, y: number } and vice versa",
	function()
		local number = luadoc.Env:get_type_by_name("number")
		local xy = luadoc.StructType:new(
			"{ x = number, y = number }",
			{ { name = "x", type = number }, { name = "y", type = number } }
		)
		local Vec2 = luadoc.AliasType:new("Vec2", xy)

		assert_assignable(Vec2, xy)
		assert_assignable(xy, Vec2)
	end
)
