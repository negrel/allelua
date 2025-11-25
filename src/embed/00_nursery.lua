-- Currently active nursery. This is consumed by Zig code.
__allelua_nursery = nil

function nursery(block)
	-- Nursery tables are edited from Zig code on submissions and completions of
	-- I/O operations.
	local n = {
		pending_count = 0,
		-- This table is populated by Zig code.
		ready = {},
	}

	local go = function(fn, ...)
		if table.is_empty(n.ready) and n.pending_count <= 0 then
			error("NurseryDead")
		end

		local co = coroutine.create(fn)
		n.ready[co] = {...}
	end

	local co = coroutine.create(block)
	n.ready[co] = {go}

	while not table.is_empty(n.ready) or n.pending_count > 0 do
		for co, args in pairs(n.ready) do
			-- Resume coroutine.
			local parent_nursery = __allelua_nursery
			__allelua_nursery = n
			local ok, err = coroutine.resume(co, table.unpack(args))
			__allelua_nursery = parent_nursery

			-- Remove entry AFTER resuming coroutine as go() may be called by the last
			-- routine. Otherwise, go() will throw a "dead nursery" error.
			n.ready[co] = nil

			-- Throw error if any.
			if not ok then error(err) end
		end

		-- Yield so runtime can poll I/O completions.
		coroutine.yield()
	end
end
