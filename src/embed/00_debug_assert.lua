debug_assert = assert
if z.mode == "debug" then
	debug_assert = function() end
end

