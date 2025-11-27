-- Start executing user main module/function. This function is called from Zig
-- code.
function __start(io, args, main)
	local main_module = Module.file(main)

	-- Load main module.
	main_module:load()

	-- Call main.main() function.
	nursery(function()
		main_module:call("main", io, args)
	end)
end

