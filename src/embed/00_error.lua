local Error = {
	__class = "allelua.Error"
}

function error(obj)
	if getmetatable(obj) == Error then
		obj.stacktrace = debug.traceback("", 2) .. obj.stacktrace
		raise(obj)
	end

	raise(setmetatable({
		message = tostring(obj),
		stacktrace = debug.traceback("", 2),
	}, Error))
end
