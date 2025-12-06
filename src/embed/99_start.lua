-- Start executing user main module/function. This function is called from Zig
-- code.
function _z.__start(args, main)
	local main_module = _z.Module.file(main)

	-- Load main module.
	main_module:load()

	-- Call main.main() function.
	_z.nursery(function()
		main_module:call("main", _z.io, args)
	end)
end

