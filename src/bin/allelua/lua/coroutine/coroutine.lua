return function(run_until, go)
	local table = require("table")
	local sync = require("sync")
	local coroutine = require("coroutine")
	local M = coroutine

	M.nursery = function(fn)
		run_until(function()
			local wg = sync.WaitGroup.new()
			local tx, rx = sync.channel()

			local id = 0
			fn(function(...)
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
			if err then error("one of nursery coroutine failed", { cause = err }) end
		end)
	end
end
