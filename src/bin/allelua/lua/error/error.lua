return function(M)
	M.new = function(...)
		local _, err = pcall(M.throw_new, ...)
		return err
	end

	setmetatable(M, {
		__call = function(_, msg, options)
			-- ignore options if number, this way old error("error msg", level) calls
			-- with level as an integer don't breaks.
			if rawtype(options) == "number" then options = nil end

			local err = msg
			if rawtype(err) == "string" then err = M.new(msg, options) end
			M.throw(err)
		end,
	})
end
