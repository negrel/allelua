import("proc")

proc.parent:on_message(function(msg)
	proc.parent:post(msg)
end)
