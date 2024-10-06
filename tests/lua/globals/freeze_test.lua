local t = require("test")

function is_frozen(t)
	local mt = __rawgetmetatable(t)
	return mt and mt.__frozen == true
end

t.test("set value in frozen table returns an error", function()
	local tab = { foo = "bar" }
	tab = freeze(tab)
	local ok, err = pcall(function()
		tab.foo = 1
	end)
	assert(is_frozen(tab))
	t.assert(not ok and type(err) == "FrozenObjectError")
end)

t.test(
	"frozen table with no metatable returns false on getmetatable",
	function()
		local tab = { foo = "bar" }
		tab = freeze(tab)
		t.assert_eq(getmetatable(tab), false)
	end
)

t.test("frozen table with metatable returns it on getmetatable", function()
	local mt = { bar = "baz" }
	mt.__index = mt
	local tab = { foo = "bar" }
	setmetatable(tab, mt)

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Freeze tab.
	tab = freeze(tab)

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Write fails.
	local ok, err = pcall(function()
		tab.bar = "foo"
	end)
	assert(not ok and type(err) == "FrozenObjectError")

	-- Write works via getmetatable.
	getmetatable(tab).bar = "foo"
	assert(tab.bar == "foo")

	-- getmetatable returns mt.
	assert(getmetatable(tab) == mt)

	-- Tab is frozen but not the mt.
	assert(is_frozen(tab))
	assert(not is_frozen(mt))
end)

t.test("frozen table and metatable returns it on getmetatable", function()
	local mt = { bar = "baz" }
	mt.__index = mt

	local tab = { foo = "bar" }
	setmetatable(tab, mt)

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Freeze tab.
	tab = freeze(tab, { metatable = true })

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Write fails.
	local ok, err = pcall(function()
		tab.bar = "foo"
	end)
	assert(not ok and type(err) == "FrozenObjectError")

	-- Write also fails via getmetatable.
	local ok, err = pcall(function()
		getmetatable(tab).bar = "foo"
	end)
	assert(not ok and type(err) == "FrozenObjectError")

	-- getmetatable returns a different metatable.
	assert(getmetatable(tab) ~= mt)
	-- But they're equals.
	t.assert_eq(getmetatable(tab), mt)

	-- Table and metatable are frozen.
	assert(is_frozen(tab))
	assert(is_frozen(getmetatable(tab)))
end)

t.test("shallow freeze of table doesn't freeze inner table", function()
	local tab = { inner = { foo = "bar" } }
	tab = freeze(tab, { shallow = true })

	assert(is_frozen(tab))
	assert(not is_frozen(tab.inner))

	tab.inner.foo = "baz"
end)

t.test("freeze of table also freeze inner table", function()
	local tab = { inner = { foo = "bar" } }
	tab = freeze(tab)

	local ok, err = pcall(function()
		tab.inner.foo = "baz"
	end)
	assert(not ok and type(err) == "FrozenObjectError")
end)
