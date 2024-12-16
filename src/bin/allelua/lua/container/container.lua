return function()
	local table = require("table")
	local math = require("math")

	local M = {}

	local MinHeap = { __type = "container.MinHeap" }
	MinHeap.__index = MinHeap
	M.MinHeap = MinHeap

	function MinHeap.new(seq)
		seq = setmetatable(seq or {}, MinHeap)
		for i = math.round(#seq / 2), 1, -1 do
			seq:_down(i)
		end
		return seq
	end

	function MinHeap:push(...)
		local init_len = #self
		table.push(self, ...)

		for i = init_len + 1, #self do
			self:_up(i)
		end
	end

	function MinHeap:pop()
		local n = #self
		self[1], self[n] = self[n], self[1]
		self:_down(1, n - 1)

		return table.pop(self)
	end

	function MinHeap:peek()
		return self[1]
	end

	function MinHeap:remove(i)
		local n = #self
		if n ~= i then
			self[i], self[n] = self[n], self[i]
			if not self:_down(i, n - 1) then self:_up(i) end
		end

		return table.pop(self)
	end

	function MinHeap:_up(i)
		while true do
			local parent_i = math.round(i / 2)
			if i == parent_i or self[parent_i] <= self[i] then break end
			self[i], self[parent_i] = self[parent_i], self[i]
			i = parent_i
		end
	end

	-- Move value at index i down in the tree unless child index is > n.
	-- n defaults to #self
	function MinHeap:_down(i, n)
		n = n or #self
		local initial_i = i

		while true do
			local child_i_left = i * 2
			if child_i_left > n or child_i_left <= 1 then break end
			local child_i = child_i_left
			local child_i_right = child_i_left + 1
			if child_i_right <= n and self[child_i_right] < self[child_i_left] then
				child_i = child_i_right
			end
			if self[child_i] >= self[i] then break end
			self[i], self[child_i] = self[child_i], self[i]
			i = child_i
		end

		-- True if i was moved down in the tree.
		return i > initial_i
	end

	local MaxHeap = { __type = "container.MaxHeap" }
	setmetatable(MaxHeap, { __index = MinHeap })
	MaxHeap.__index = MaxHeap
	M.MaxHeap = MaxHeap

	function MaxHeap.new(seq)
		seq = setmetatable(seq or {}, MaxHeap)
		for i = math.round(#seq / 2), 1, -1 do
			seq:_down(i)
		end
		return seq
	end

	function MaxHeap:_up(i)
		while true do
			local parent_i = math.round(i / 2)
			if i == parent_i or self[parent_i] >= self[i] then break end
			self[i], self[parent_i] = self[parent_i], self[i]
			i = parent_i
		end
	end

	function MaxHeap:_down(i, n)
		n = n or #self
		local initial_i = i

		while true do
			local child_i_left = i * 2
			if child_i_left > n or child_i_left <= 1 then break end
			local child_i = child_i_left
			local child_i_right = child_i_left + 1
			if child_i_right <= n and self[child_i_right] > self[child_i_left] then
				child_i = child_i_right
			end
			if self[child_i] <= self[i] then break end
			self[i], self[child_i] = self[child_i], self[i]
			i = child_i
		end

		-- True if i was moved down in the tree.
		return i > initial_i
	end

	return M
end
