package.preload["time"] = function()
	return {
		sleep = function(io, secs)
			io:sleep(secs * 1000)
			local ok, err = coroutine.yield()
			if not ok then error(err) end
		end
	}
end

