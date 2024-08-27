local time = require('time')

for i = 1, 1000 do
	go(function()
		time.sleep(i * time.millisecond)
		print("goroutine", i, "done")
	end)
end
