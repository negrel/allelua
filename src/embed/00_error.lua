local Error = {}

function _z.error(obj)
	if getmetatable(obj) == Error then
		obj.stacktrace = debug.traceback("", 2) .. obj.stacktrace
		_z.raise(obj)
	end

	_z.raise(setmetatable({
		message = tostring(obj),
		stacktrace = debug.traceback("", 2),
	}, Error))
end
