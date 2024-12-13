return function(BigInt, lcm, gcd, gcd_lcm)
	local M = require("math")
	M.BigInt = BigInt

	M.round = function(n)
		return M.floor(n + 0.5)
	end

	M.lcm = lcm
	M.gcd = gcd
	M.gcd_lcm = gcd_lcm
end
