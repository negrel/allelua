-- Start executing user main module/function. This function is called from Zig
-- code.
function __start(io, args, main)
	local main_g = {}
	local global = {
		error = error,
		ipairs = ipairs,
		pairs = pairs,
		pcall = pcall,
		print = print,
		type = type,
	}
	setmetatable(global, {
		__newindex = function(_, k, v)
			main_g[k] = v
		end,
		__index = function(_, k, v)
			return main_g[k]
		end,
	})

	setfenv(main, global)()

	-- User main module defined a main function.
	if type(main_g["main"]) == "function" then
		setfenv(main_g["main"], global)(io, args)
	end
end

