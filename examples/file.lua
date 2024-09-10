local os = require("os")
local io = require("io")
local string = require("string")
local errors = require("errors")


-- Open stdin with read permissions.
local stdin = os.File.open("/home/anegrel/TODO.md", "r")

local buf = ""
local ok, err = pcall(function()
	while true do
		local byte, err = stdin:read_exact(1)
		if byte == nil or byte == "" then
			print(err, type(err))
			break
		elseif byte == "\n" then
			print(buf)
			buf = ""
		else
			buf = buf .. byte
		end
	end
end)

print(err, type(err))
