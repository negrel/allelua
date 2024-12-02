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
