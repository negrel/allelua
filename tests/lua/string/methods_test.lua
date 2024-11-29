local string = require("string")
local t = require("test")

t.test("string.contains('allelua', 'lua') returns true", function()
	assert(string.contains("allelua", "lua"))
end)

t.test("string.contains('allelua', 'all') returns true", function()
	assert(string.contains("allelua", "all"))
end)

t.test("string.contains('allelua', 'lel') returns true", function()
	assert(string.contains("allelua", "lel"))
end)

t.test("string.contains('allelua', '') returns true", function()
	assert(string.contains("allelua", ""))
end)

t.test("string.contains('allelua', 'allelua') returns true", function()
	assert(string.contains("allelua", "allelua"))
end)

t.test("string.contains('allelua', 'ALLELUA') returns false", function()
	assert(not string.contains("allelua", "ALLELUA"))
end)

t.test("string.contains('allelua', 'alleluaa') returns false", function()
	assert(not string.contains("allelua", "alleluaa"))
end)

t.test("string.contains('allelua', 'Lua') returns false", function()
	assert(not string.contains("allelua", "Lua"))
end)

t.test(
	"string.split('foo,bar,baz', ',') return { 'foo', 'bar', 'baz' }",
	function()
		t.assert_eq(("foo,bar,baz"):split(","), { "foo", "bar", "baz" })
	end
)

t.test("string.find('foobarbaz', 'bar') return 4, 6", function()
	local str = "foobarbaz"
	local substr, i, j = str:find("bar")
	t.assert_eq(i, 4)
	t.assert_eq(j, 6)
	t.assert_eq(str:slice(i, j), "bar")
	t.assert_eq(substr, "bar")
end)

t.test("string.find('foobarbaz.', '.') return 9, 9", function()
	local str = "foobarbaz."
	local substr, i, j = str:find(".")
	t.assert_eq(i, 10)
	t.assert_eq(j, 10)
	t.assert_eq(substr, ".")
end)

t.test("string.find_iter('foobarbaz', 'b') finds (4, 6) and (7, 8)", function()
	local str = "foobarbaz"
	local expected = { "bar", "baz" }
	local count = 0
	local re = string.Regex.new("ba.")
	for substr, i, j in str:find_iter(re) do
		count = count + 1
		t.assert_eq(str:slice(i, j), expected[count])
		t.assert_eq(substr, expected[count])
	end

	assert(count == 2)
end)

t.test("string.replace('foobarbar', 'a', 'A') returns foobArbaz)", function()
	local str = "foobarbaz"
	t.assert_eq(str:replace("a", "A"), "foobArbaz")
end)

t.test("string.replace('foobarbar', 'a', 'A', 2) returns foobArbAz)", function()
	local str = "foobarbaz"
	t.assert_eq(str:replace("a", "A", 2), "foobArbAz")
end)

t.test(
	"string.replace_all('foobarbar', 'a', 'A') returns foobArbAz)",
	function()
		local str = "foobarbaz"
		t.assert_eq(str:replace_all("a", "A"), "foobArbAz")
	end
)

t.test(
	"string.replace_all('foobarbar', /o+/, 'a') returns faabarbaz)",
	function()
		local str = "foobarbaz"
		t.assert_eq(str:replace_all(string.Regex.new("o+"), "a"), "fabarbaz")
	end
)

t.test(
	"string.captures(/(?<month>[0-9]{2})/) returns {{ '01', 5, 7, name = 'month' }}",
	function()
		local str = "bar-01-foo"
		local re = string.Regex.new("(?<month>[0-9]{2})")
		local captures = str:captures(re)
		t.assert_eq(captures[1], {
			"01",
			5,
			6,
			"month",
			match = "01",
			start = 5,
			["end"] = 6,
			name = "month",
		})
		t.assert_eq(captures.month, {
			"01",
			5,
			6,
			"month",
			match = "01",
			start = 5,
			["end"] = 6,
			name = "month",
		})
	end
)

t.test(
	"string.captures_iter(/(?<month>[0-9]{2})/) returns {{ '01', 5, 7, name = 'month' }}",
	function()
		local str = "bar-01-foo-0-02"
		local re = string.Regex.new("(?<month>[0-9]{2})")

		local expected = {
			{
				"01",
				5,
				6,
				"month",
				match = "01",
				start = 5,
				["end"] = 6,
				name = "month",
			},
			{
				"02",
				14,
				15,
				"month",
				match = "02",
				start = 14,
				["end"] = 15,
				name = "month",
			},
		}

		local count = 0
		for captures in str:captures_iter(re) do
			count = count + 1
			t.assert_eq(captures[1], expected[count])
			t.assert_eq(captures.month, expected[count])
		end

		t.assert_eq(count, 2)
	end
)

t.test("slice within bounds return substring", function()
	local str = "Hello world!"
	local substr = str:slice(3, 6)
	assert(substr == "llo ")
end)

t.test(
	"slice from within bounds to out of bound return substring from start up to end",
	function()
		local str = "Hello world!"
		local substr = str:slice(3, 64)
		assert(substr == "llo world!")
	end
)

t.test("slice from out of bound to in bound returns empty string", function()
	local str = "Hello world!"
	local substr = str:slice(64, 3)
	assert(substr == "")
end)

t.test("slice from 0 returns all string", function()
	local str = "Hello world!"
	local substr = str:slice(0)
	assert(substr == str)
end)

t.test("slice from 1 returns all string", function()
	local str = "Hello world!"
	local substr = str:slice(1)
	assert(substr == str)
end)

t.test("slice from -2 returns last 2 bytes of string", function()
	local str = "Hello world!"
	local substr = str:slice(-2)
	assert(substr == "d!")
end)

t.test("slice from -2 to -1 returns last 2 bytes of string", function()
	local str = "Hello world!"
	local substr = str:slice(-2, -1)
	assert(substr == "d!")
end)

t.test(
	"slice from -2 to -2 returns single byte before last byte of string",
	function()
		local str = "Hello world!"
		local substr = str:slice(-2, -2)
		assert(substr == "d")
	end
)

t.test("slice from -2 to 0 returns empty string", function()
	local str = "Hello world!"
	local substr = str:slice(-2, 0)
	assert(substr == "")
end)

t.test('slice from -12 to 5 returns "Hello"', function()
	local str = "Hello world!"
	local substr = str:slice(-12, 5)
	assert(substr == "Hello")
end)

t.test("slice from -16 to -1 returns all string", function()
	local str = "Hello world!"
	local substr = str:slice(-16, -1)
	assert(substr == str)
end)

t.test("slice from -16 to -13 returns empty", function()
	local str = "Hello world!"
	local substr = str:slice(-16, -13)
	assert(substr == "")
end)
