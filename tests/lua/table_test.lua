local t = require("test")
local table = require("table")

t.test("reverse {1, 2, 3} returns {3, 2, 1}", function()
	local rev = table.reverse { 1, 2, 3 }
	t.assert_eq(rev, { 3, 2, 1 })
end)

t.test("reverse {1, 2} returns {2, 1}", function()
	local rev = table.reverse { 1, 2 }
	t.assert_eq(rev, { 2, 1 })
end)

t.test("slice with no args create a shallow copy of sequence", function()
	-- selene: allow(mixed_table)
	local tab = { 1, 2, 3, foo = "bar" }
	local slice = table.slice(tab)

	t.assert_eq(slice, { 1, 2, 3 })
end)

t.test("slice -1 returns last element of {1, 2, 3}", function()
	local tab = { 1, 2, 3 }
	local slice = table.slice(tab, -1)

	t.assert_eq(slice, { 3 })
end)

t.test("slice -3 returns all elements of {1, 2, 3}", function()
	local tab = { 1, 2, 3 }
	local slice = table.slice(tab, -3)

	t.assert_eq(slice, tab)
end)

t.test("slice -4 returns all elements of {1, 2, 3}", function()
	local tab = { 1, 2, 3 }
	local slice = table.slice(tab, -4)

	t.assert_eq(slice, tab)
end)

t.test("slice -4 to -3 returns first element of {1, 2, 3}", function()
	local tab = { 1, 2, 3 }
	local slice = table.slice(tab, -4, -3)

	t.assert_eq(slice, { 1 })
end)

t.test("slice -4 to 64 returns all elements of {1, 2, 3}", function()
	local tab = { 1, 2, 3 }
	local slice = table.slice(tab, -4, 64)

	t.assert_eq(slice, tab)
end)

t.test("slice -2 to 64 returns last two elements of {1, 2, 3}", function()
	local tab = { 1, 2, 3 }
	local slice = table.slice(tab, -2, 64)

	t.assert_eq(slice, { 2, 3 })
end)

t.test("for_eachi calls function for each element in sequence", function()
	-- selene: allow(mixed_table)
	local tab = { 1, 2, 3, foo = "bar" }
	local tab2 = {}
	table.ifor_each(tab, function(_i, v)
		table.push(tab2, v)
	end)

	t.assert_eq(tab2, { 1, 2, 3 })
end)
