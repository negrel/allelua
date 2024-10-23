local t = require("test")

t.test("infinite loop", function()
	while true do
		-- noop
	end
end)
