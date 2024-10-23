local error = require("error")

local t = require("test")

t.test("err:is(err) returns true", function()
	local _, err = pcall(error, "foo")
	assert(err:is(err))
end)

t.test("err:is(cause) returns true", function()
	local _, cause = pcall(error, "cause error")
	local _, err = pcall(error, "foo", { cause = cause })
	assert(err:is(cause))
end)

t.test(
	"error.is of two RuntimeError of Uncategorized kind returns true when error message is the same",
	function()
		local _, foo1 = pcall(error, "foo")
		local _, foo2 = pcall(error, "foo")
		assert(foo1:is(foo2))
		assert(foo2:is(foo1))
	end
)

t.test(
	"error.is of two RuntimeError of Uncategorized kind returns false when error message is different",
	function()
		local _, foo = pcall(error, "foo")
		local _, bar = pcall(error, "bar")
		assert(not foo:is(bar))
	end
)
