return function(run_until, go)
	local table = require("table")
	local sync = require("sync")
	local coroutine = require("coroutine")
	local M = coroutine

	M.nursery = function(fn)
		return run_until(function()
			local wg = sync.WaitGroup.new()
			local tx, rx = sync.channel()

			local id = 0
			go(fn, function(...)
				id = id + 1
				local args = { ... }
				wg:add(1)
				local abort = go(function()
					local ok, err = pcall(table.unpack(args))
					if not ok then tx:send(err) end
					wg:done()
				end)

				return function()
					abort()
					wg:done()
				end
			end)

			go(function()
				wg:wait()
				tx:close()
			end)

			local err = rx:recv()
			if err then
				error(("nursery coroutine %d failed"):format(id), {
					type = "coroutine.NurseryError",
					kind = "CoroutineError",
					cause = err,
					id = id,
				})
			end
		end)
	end
end
