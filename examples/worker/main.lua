import "proc"
import "time"

local w = proc.Worker.new("./worker.lua")

local batch = 16

for _ = 1, batch do
	w:post(("ping"):rep(3000))
end

local i = 0

w:on_message(function(msg)
	i = i + 1
	if i <= 100000 then
		w:post(msg)
	elseif i - batch == 100000 then
		w:terminate()
	end
end)
