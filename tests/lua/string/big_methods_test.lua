local string = require("string")
local t = require("test")

t.test("slice within bounds return substring", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(3, 6)
	assert(substr:eq("llo "))
end)

t.test(
	"slice from within bounds to out of bound return substring from start up to end",
	function()
		local str = string.Big.fromstring("Hello world!")
		local substr = str:slice(3, 64)
		assert(substr:eq("llo world!"))
	end
)

t.test("slice from out of bound to in bound returns empty string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(64, 3)
	assert(substr:eq(""))
end)

t.test("slice from 0 returns all string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(0)
	assert(substr == str)
end)

t.test("slice from 1 returns all string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(1)
	assert(substr == str)
end)

t.test("slice from -2 returns last 2 bytes of string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(-2)
	assert(substr:eq("d!"))
end)

t.test("slice from -2 to -1 returns last 2 bytes of string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(-2, -1)
	assert(substr:eq("d!"))
end)

t.test(
	"slice from -2 to -2 returns single byte before last byte of string",
	function()
		local str = string.Big.fromstring("Hello world!")
		local substr = str:slice(-2, -2)
		assert(substr:eq("d"))
	end
)

t.test("slice from -2 to 0 returns empty string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(-2, 0)
	assert(substr:eq(""))
end)

t.test('slice from -12 to 5 returns "Hello"', function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(-12, 5)
	assert(substr:eq("Hello"))
end)

t.test("slice from -16 to -1 returns all string", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(-16, -1)
	assert(substr == str)
end)

t.test("slice from -16 to -13 returns empty", function()
	local str = string.Big.fromstring("Hello world!")
	local substr = str:slice(-16, -13)
	assert(substr:eq(""))
end)
