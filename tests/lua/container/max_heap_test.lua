local container = require("container")
local math = require("math")
local t = require("test")

t.test("MaxHeap.new { 2, 3, 1 } heapify sequence", function()
	local h = container.MaxHeap.new { 2, 3, 1 }

	t.assert_eq(#h, 3)
	t.assert_eq(h:peek(), 3)
	t.assert_eq(h:pop(), 3)
	t.assert_eq(#h, 2)
	t.assert_eq(h:peek(), 2)
	t.assert_eq(h:pop(), 2)
	t.assert_eq(#h, 1)
	t.assert_eq(h:peek(), 1)
	t.assert_eq(h:pop(), 1)
	t.assert_eq(#h, 0)
end)

t.test("MaxHeap.new { 2, 3, 1, 0 } heapify sequence", function()
	local h = container.MaxHeap.new { 2, 3, 1, 0 }

	t.assert_eq(#h, 4)
	t.assert_eq(h:peek(), 3)
	t.assert_eq(h:pop(), 3)
	t.assert_eq(#h, 3)
	t.assert_eq(h:peek(), 2)
	t.assert_eq(h:pop(), 2)
	t.assert_eq(#h, 2)
	t.assert_eq(h:peek(), 1)
	t.assert_eq(h:pop(), 1)
	t.assert_eq(#h, 1)
	t.assert_eq(h:peek(), 0)
	t.assert_eq(h:pop(), 0)
	t.assert_eq(#h, 0)
end)

t.test("MaxHeap.push -7 into { 3, 2, 1 }", function()
	local h = container.MaxHeap.new { 3, 2, 1 }

	t.assert_eq(#h, 3)

	h:push(-7)
	t.assert_eq(#h, 4)
	t.assert_eq(h:peek(), 3)
	t.assert_eq(h:pop(), 3)

	t.assert_eq(#h, 3)
	t.assert_eq(h:pop(), 2)
	t.assert_eq(#h, 2)
	t.assert_eq(h:pop(), 1)
	t.assert_eq(#h, 1)
	t.assert_eq(h:pop(), -7)
	t.assert_eq(#h, 0)
end)
