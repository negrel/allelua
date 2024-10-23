local coroutine = require("coroutine")
local time = require("time")

-- Structured concurrency using nursery.
print("starting concurrent work in nursery...")
coroutine.nursery(function(go)
	go(function()
		print("  sleeping 1s in 1st coroutine")
		time.sleep(time.second)
		print("  1st coroutine done")
	end)

	go(function()
		print("  sleeping 0.5s in 2nd coroutine")
		time.sleep(time.second / 2)
		print("  2nd coroutine done")
	end)
end)
print("nursery work done")

print()

-- Unlike goroutines, when an error is thrown, it is propagated.
print("starting concurrent work in nursery...")
local ok, err = pcall(function()
	coroutine.nursery(function(go)
		go(function()
			print("  sleeping 1s in 1st coroutine")
			time.sleep(time.second)
			print("  1st coroutine done")
		end)

		go(function()
			print("  sleeping 0.5s in 2nd coroutine")
			time.sleep(time.second / 2)
			-- throw an error.
			error("error because foo bar baz")
		end)
	end)
end)
print("nursery result:", ok, err)
