return function(BigInt)
	local M = require("math")
	M.BigInt = BigInt

	M.round = function(n)
		return M.floor(n + 0.5)
	end
end
