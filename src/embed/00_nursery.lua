-- Nursery is a structured concurrency primitive. This class is part of the
-- runtime but user code can't access it directly. Zig runtime's code read and
-- write to Nursery instances.
_z.Nursery = {
	-- Currently running nursery instance.
	running = nil,
}
local Nursery = _z.Nursery
Nursery.__index = Nursery

function Nursery:new()
	local n = {
		parent = Nursery.running,
		co = coroutine.running(),
		ready = {},
		pending = {},
	}
	return setmetatable(n, Nursery)
end

--- Resume all ready routines.
function Nursery:resume()
	for co, args in pairs(self.ready) do
		--debug_assert(Nursery.running == self.parent)
		Nursery.running = self
		local ok, err = coroutine.resume(co, table.unpack(args))
		Nursery.running = self.parent

		-- Remove from ready table.
		self.ready[co] = nil

		-- An error occurred forward it.
		if not ok then self:error(err) end

		if coroutine.status(co) == "suspended" then
			self.pending[co] = true
		end
	end
end

--- Returns whether nursery is dead: it has 0 pending and ready I/O operations.
function Nursery:is_dead()
	return table.is_empty(self.pending) and table.is_empty(self.ready)
end

function Nursery:error(err)
	-- TODO: cancel pending tasks.
	_z.raise(err)
end

--- Spawn a new child coroutine and execute it as soon as possible.
function Nursery:spawn(fn, ...)
	if Nursery.running ~= self then error("NurseryDead") end
	self.ready[coroutine.create(fn)] = {...}
end

--- Starts a nursery block that returns when all coroutines have returned.
function nursery(block)
	local n = Nursery:new()

	-- Prepare nursery block to be executed.
	local co = coroutine.create(block)
	n.ready[co] = { function(...) n:spawn(...) end }

	while true do
		n:resume()

		if not table.is_empty(n.pending) then
			coroutine.yield()
		elseif table.is_empty(n.ready) then
			break
		end
	end
end

