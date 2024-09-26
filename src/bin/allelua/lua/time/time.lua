local channel = require("sync").channel

return function(M)
	M.after = function(dur)
		local tx, rx = channel()
		return rx, go(function()
			M.sleep(dur)
			tx:close()
		end)
	end
end
