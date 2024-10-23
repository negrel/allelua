local channel = require("sync").channel

return function(M)
	--- After waits for the duration to elapse and then closes the current time
	--- on the returned channel.
	--- @param dur time.Duration
	--- @returns sync.ChannelReceiver
	M.after = function(go, dur)
		local tx, rx = channel()
		return rx, go(function()
			M.sleep(dur)
			tx:close()
		end)
	end
end
