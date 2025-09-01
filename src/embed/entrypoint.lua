return function(...)
	coroutine.nursery(function(n)
		n._wake = function(...)
			print("root wake", ...)
			print()
			print()
			print()
		end

		local main = loadfile("main.lua")
		main()
	end)
end
