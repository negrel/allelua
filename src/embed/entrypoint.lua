return function(...)
	coroutine.nursery(function(n)
		local main = loadfile("main.lua")
		main()
	end)
end
