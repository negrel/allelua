local t = require("test")
local table = require("table")

t.test("can't set value in frozen table", function()
	local tab = { foo = "bar" }
	tab = table.freeze(tab)
	local ok, err = pcall(function()
		tab.foo = 1
	end)
	t.assert(not ok and type(err) == "FrozenTableError")
end)

t.test("frozen table with no metatable returns nil on getmetatable", function()
	local tab = { foo = "bar" }
	tab = table.freeze(tab)
	t.assert_eq(getmetatable(tab), false)
end)

t.test("frozen table with metatable returns it on getmetatable", function()
	local mt = { bar = "baz" }
	mt.__index = mt
	local tab = { foo = "bar" }
	setmetatable(tab, mt)

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Freeze tab.
	tab = table.freeze(tab)

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Write fails.
	local ok, err = pcall(function()
		tab.bar = "foo"
	end)
	assert(not ok and type(err) == "FrozenTableError")

	-- Write works via getmetatable.
	getmetatable(tab).bar = "foo"
	assert(tab.bar == "foo")

	-- getmetatable returns mt.
	assert(getmetatable(tab) == mt)

	-- Tab is frozen but not the mt.
	assert(table.is_frozen(tab))
	assert(not table.is_frozen(mt))
end)

t.test("frozen table and metatable returns it on getmetatable", function()
	local mt = { bar = "baz" }
	mt.__index = mt

	local tab = { foo = "bar" }
	setmetatable(tab, mt)

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Freeze tab.
	tab = table.freeze(tab, { metatable = true })

	-- Read from __index.
	assert(tab.bar == "baz")

	-- Write fails.
	local ok, err = pcall(function()
		tab.bar = "foo"
	end)
	assert(not ok and type(err) == "FrozenTableError")

	-- Write also fails via getmetatable.
	local ok, err = pcall(function()
		getmetatable(tab).bar = "foo"
	end)
	assert(not ok and type(err) == "FrozenTableError")

	-- getmetatable returns a different metatable.
	assert(getmetatable(tab) ~= mt)
	-- But they're equals.
	t.assert_eq(getmetatable(tab), mt)

	-- Table and metatable are frozen.
	assert(table.is_frozen(tab))
	assert(table.is_frozen(getmetatable(tab)))
end)
