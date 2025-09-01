local coroutine = require("coroutine")
local table = require("table")

--- Nursery defines the primitive type of structured concurrency. A nursery is
--- a block of instruction (function in Lua), that can fork new thread
--- of code that runs concurrently and join before the block returns.
---
--- ```lua
--- coroutine.nursery(function(n)
---   n:go(sleep, 1)
---   n:go(sleep, 2)
---   n:go(sleep, 3)
--- end)
--- -- All routines are done here.
--- ```
local Nursery = { _nursery = true, __metatable = false }
Nursery.__index = Nursery

--- Creates a coroutine ready to execute provided block.
function Nursery:go(block)
	local co = coroutine.create(block)
	self._ready[co] = co
end

function Nursery:_poll()
	local nursery = coroutine._nursery
	coroutine._nursery = self

	-- Resume all ready routines.
	while not table.empty(self._ready) do
		for co in pairs(self._ready) do
			local ok, val = coroutine.resume(co, self)

			-- Forward error if any.
			if not ok then error(val) end

			-- Move to pending set until completion move it back to ready set.
			if type(val) == "table" and val._nursery == true then
				self._ready[co] = nil
				self._pending[co] = co
			else
				if coroutine.status(co) == "dead" then
					self._ready[co] = nil -- remove coroutine
				else
					error("can't yield across nursery boundary")
				end
			end
		end
	end

	coroutine._nursery = nursery
end

function Nursery:_wake(co)
	if self._pending[co] then
		self._pending[co] = nil
		self._ready[co] = co
		-- Recursively wake routine in parent nursery.
		self._parent:_wake(self._co)
	else
		error("can't wake unknown coroutine")
	end
end

-- Root nursery mock.
coroutine._nursery = setmetatable({
	_nursery = true,
	_wake = function() end,
}, Nursery)

function coroutine.nursery(block)
	local co = coroutine.create(block)
	local nu = setmetatable({
		-- Parent nursery if any.
		_parent = coroutine._nursery,
		_co = coroutine.running(),
		-- Routines ready to be executed on next poll.
		_ready = { [co] = co },
		-- Routines waiting for I/O completion.
		_pending = {},
	}, Nursery)

	while true do
		-- Poll ready routines.
		nu:_poll()

		-- No more work to do, nursery is done.
		if table.empty(nu._pending) then return end

		-- All routines are pending, perform an async yield.
		coroutine.yield(nu._parent)
	end
end


