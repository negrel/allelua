local t = require("test")
local table = require("table")

t.test("push 1, 2, 3 to { 3, 2, 1 } produces { 3, 2, 1, 1, 2, 3 }", function()
	local tab = { 3, 2, 1 }
	table.push(tab, 1, 2, 3)
	t.assert_eq(tab, { 3, 2, 1, 1, 2, 3 })
end)

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

t.test("binary search for 2 in { 1, 2, 3 } returns 2", function()
	local tab = { 1, 2, 3 }
	local i = table.binary_search(tab, 2)
	assert(i == 2)
end)

t.test(
	"binary search for 2 in { 0,1, 2, 3, 4, 5, 6, 7, 8, 9 } returns 3",
	function()
		local tab = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 }
		local i = table.binary_search(tab, 2)
		assert(i == 3)
	end
)

t.test(
	"binary search for 100 in { 0,1, 2, 3, 4, 5, 6, 7, 8, 9 } returns nil",
	function()
		local tab = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 }
		local i = table.binary_search(tab, 100)
		assert(i == nil)
	end
)

t.test("flatten flat sequence returns a copy", function()
	local tab = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 }
	local flat = table.flat(tab)
	assert(tab ~= flat)
	t.assert_eq(tab, flat)
end)

t.test(
	"flatten { { 1, 2, 3 }, { 4, 5, 6 } } returns { 1, 2, 3, 4, 5, 6 }",
	function()
		local tab = { { 1, 2, 3 }, { 4, 5, 6 } }
		local flat = table.flat(tab)
		t.assert_eq(flat, { 1, 2, 3, 4, 5, 6 })
	end
)

t.test(
	"flatten { { { 1, 2, 3 }, { 4, 5, 6 } } } returns { { 1, 2, 3 }, { 4, 5, 6 } }",
	function()
		local tab = { { { 1, 2, 3 }, { 4, 5, 6 } } }
		local flat = table.flat(tab)
		t.assert_eq(flat, { { 1, 2, 3 }, { 4, 5, 6 } })
	end
)

t.test(
	"flatten with no max depth of 2 { { { 1, 2, 3 }, { 4, 5, 6 } } } returns { 1, 2, 3, 4, 5, 6 }",
	function()
		local tab = { { { 1, 2, 3 }, { 4, 5, 6 } } }
		local flat = table.flat(tab, 2)
		t.assert_eq(flat, { 1, 2, 3, 4, 5, 6 })
	end
)

t.test("delete from 1 to 1 in { 1, 2, 3 } produces { 2, 3 }", function()
	local tab = { 1, 2, 3 }
	local removed = table.delete(tab, 1)
	t.assert_eq(tab, { 2, 3 })
	t.assert_eq(removed, { 1 })
end)

t.test("delete from 1 to 2 in { 1, 2, 3 } produces { 3 }", function()
	local tab = { 1, 2, 3 }
	local removed = table.delete(tab, 1, 2)
	t.assert_eq(tab, { 3 })
	t.assert_eq(removed, { 1, 2 })
end)

t.test("delete from 1 to 3 in { 1, 2, 3 } produces { }", function()
	local tab = { 1, 2, 3 }
	local removed = table.delete(tab, 1, 3)
	t.assert_eq(tab, {})
	t.assert_eq(removed, { 1, 2, 3 })
end)

t.test("delete from 1 to -1 in { 1, 2, 3 } produces { }", function()
	local tab = { 1, 2, 3 }
	local removed = table.delete(tab, 1, -1)
	t.assert_eq(tab, {})
	t.assert_eq(removed, { 1, 2, 3 })
end)

t.test("dedup { 1, 2, 1 } produces { 1, 2, 1 }", function()
	local tab = { 1, 2, 1 }
	table.dedup(tab)
	t.assert_eq(tab, { 1, 2, 1 })
end)

t.test("dedup { 1, 1, 1 } produces { 1 }", function()
	local tab = { 1, 1, 1 }
	table.dedup(tab)
	t.assert_eq(tab, { 1 })
end)

t.test("dedup { 1, 1, 2, 1 } produces { 1, 2, 1 }", function()
	local tab = { 1, 1, 2, 1 }
	table.dedup(tab)
	t.assert_eq(tab, { 1, 2, 1 })
end)
