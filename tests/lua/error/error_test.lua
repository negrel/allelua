local error = require("error")

local t = require("test")

t.test('type(err) returns "error"', function()
	local _, err = pcall(error, "foo")
	assert(type(err) == "error")
end)

t.test("type(err) returns provided type", function()
	local _, err = pcall(error, "foo", { type = "MyError" })
	assert(type(err) == "MyError")
end)
