return function()
	local math = require("math")
	local buffer = require("string.buffer")
	local io = require("io")
	local profile = require("jit.profile")

	local M = {}
	_G.perf = M

	M.cpu = {}
	function M.cpu:start_profiler(dur)
		assert(self.data == nil, "cpu profiler already started")

		if type(dur) == "time.Duration" then dur = dur:tonumber() * 1000 end
		self.data = {}
		profile.start("fi" .. tostring(dur), function(thread, samples, _vmstate)
			local dump = profile.dumpstack(thread, "fZ;", math.huge)
			self.data[dump] = (self.data[dump] or 0) + samples
		end)
	end

	function M.cpu:stop_profiler(w)
		profile.stop()
		local result = self.data
		self.data = nil

		local buf = buffer.new()

		for entry, samples in pairs(result) do
			buf:put(entry)
			buf:put(" ")
			buf:put(samples)
			buf:put("\n")

			io.write_all(w, buf, true)
			buf:reset()
		end

		return result
	end

	return M
end
