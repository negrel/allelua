local os = require("os")
local string = require("string")

local tr_proc = os.exec("tr", { args = { "[a-z]", "[A-Z]" }, stdin = "piped" })

local ls_proc = os.exec("ls", { stdout = "piped" })

-- Emulate a pipe.
do
	local buf = string.buffer.new()
	local ls_out = ls_proc:stdout()
	local tr_in = tr_proc:stdin()

	while true do
		-- Read from ls.
		local read = ls_out:read(buf, 4096)
		if read == 0 then
			tr_in:close()
			break
		end

		-- Write to tr.
		tr_in:write_all(buf)
	end
end

ls_proc:wait()
print("ls done")
tr_proc:wait()
print("tr done")
