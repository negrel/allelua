local co = coroutine.create(function()
	coroutine.yield(1)
	coroutine.yield(2)
	coroutine.yield(3)
	return 4
end)

print(coroutine.resume(co))
print(coroutine.resume(co))
print(coroutine.resume(co))
print(coroutine.resume(co))
print(coroutine.resume(co))
