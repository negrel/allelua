package.preload["time"] = function()
	return {
		sleep = function(io, secs)
			io:sleep(secs * 1000)
			coroutine.yield()
		end
	}
end
