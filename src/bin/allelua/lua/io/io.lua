return function(M)
	local string = require("string")

	M.copy = function(reader, writer)
		if reader.write_to then
			while true do
				local n = reader:write_to(writer)
				if n == 0 then break end
			end
		else
			local buf = string.buffer.new(4096)
			while true do
				local read = reader:read(buf)
				if read == 0 then break end
				writer:write_all(buf)
			end
		end
	end
end
