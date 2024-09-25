local M = {}

M.add = function(a, b)
	return a + b
end

M.div = function(a, b)
	if b == 0 then error("can't divise by 0") end
	return a / b
end

return M
