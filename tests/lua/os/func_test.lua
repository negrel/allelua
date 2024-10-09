local os = require("os")
local t = require("test")

if os.os_name == "linux" then
	t.test("current_dir", function()
		t.assert_eq(os.current_dir(), os.env_vars.PWD)
	end)

	t.test("temp_dir", function()
		t.assert_eq(os.temp_dir(), "/tmp")
	end)
end
