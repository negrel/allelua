package.preload["time"] = function()
	return {
		sleep = function(io, ms)
			io:sleep(ms)
			coroutine.yield()
		end
	}
end
