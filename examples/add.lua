--@ {
--    add: <T>(T) -> T,
--    div: <T>(T) -> T
--  }
local M = {}

--- This is a lua doc comment
--@ <T>(T) -> T
function add(a)
	return a
end

M.add = function(a, b)
	return a + b
end

M.div = function(a, b)
	if b == 0 then error("can't divise by 0") end
	return a / b
end

return M
