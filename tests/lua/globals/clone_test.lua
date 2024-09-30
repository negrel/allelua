local t = require("test")

t.test("clone table and its metatable", function()
	local mt = { extra = 1 }
	mt.__index = mt
	local tab = { foo = "bar", bool = true, num = 1, inner = {} }
	tab.inner.parent = tab
	tab.inner[tab] = "parent"

	setmetatable(tab, mt)

	local tab_clone = clone(tab)

	-- Metatable was copied.
	assert(getmetatable(tab_clone) == getmetatable(tab))

	-- Table are different...
	assert(tab ~= tab_clone, "clone returned same table")

	-- ...but contains same data.
	t.assert_eq(tab.foo, tab_clone.foo)
	t.assert_eq(tab.bool, tab_clone.bool)
	t.assert_eq(tab.num, tab_clone.num)
	t.assert_eq(tab.extra, tab_clone.extra)
	t.assert_eq(tab.inner[tab], tab_clone.inner[tab])
	assert(tab.inner == tab_clone.inner)

	-- Self referential data points to old data.
	assert(tab == tab_clone.inner.parent)
end)

t.test("clone table without its associated metatable", function()
	local mt = { extra = 1 }
	mt.__index = mt
	local tab = { foo = "bar" }

	setmetatable(tab, mt)

	local tab_clone = clone(tab, { metatable = { skip = true } })

	-- Clone metatable is nil.
	assert(getmetatable(tab_clone) == nil)

	-- Table are different...
	assert(tab ~= tab_clone, "clone returned same table")
end)

t.test("deep clone table", function()
	local mt = { extra = 1 }
	mt.__index = mt
	local tab = { foo = "bar", bool = true, num = 1, inner = {} }
	tab.inner.parent = tab
	tab.inner[tab] = "parent"

	setmetatable(tab, mt)

	local tab_clone = clone(tab, { deep = true })

	-- Metatable was copied.
	assert(getmetatable(tab_clone) == getmetatable(tab))

	-- Table are different...
	assert(tab ~= tab_clone, "clone returned same table")

	-- ...but contains same data.
	t.assert_eq(tab.foo, tab_clone.foo)
	t.assert_eq(tab.bool, tab_clone.bool)
	t.assert_eq(tab.num, tab_clone.num)
	t.assert_eq(tab.extra, tab_clone.extra)
	t.assert_eq(tab.inner[tab], tab_clone.inner[tab_clone])
	assert(tab.inner ~= tab_clone.inner)

	-- Self referential data points to new data.
	assert(tab_clone == tab_clone.inner.parent)
end)

t.test("clone table using __clone metamethod", function()
	local mt = {
		__clone = function(tab, opts)
			assert(opts.deep == false)
			t.assert_eq(opts.metatable, {})

			local clone = { count = (tab.count or 0) + 1 }
			opts.replace[tab] = clone

			setmetatable(clone, getmetatable(tab))

			return clone
		end,
	}

	local tab = {}
	setmetatable(tab, mt)

	local tab_clone = clone(tab)
	assert(tab_clone.count == 1)

	tab_clone = clone(tab_clone)
	assert(tab_clone.count == 2)
end)
