local byte = require("byte")
local os = require("os")
local table = require("table")

local t = { 1, 2 }
table.push(t, t)

setmetatable(t, {
	__clone = function(t, opts)
		local result = {}
		opts.replace[t] = result

		for k, v in pairs(t) do
			result[clone(k, opts)] = clone(v, opts)
		end

		setmetatable(result, getmetatable(t))
		return result
	end,
})

print(t, clone(t))

local buf = byte.Buffer.fromstring("foo")
print(type(buf), buf, clone(buf))
