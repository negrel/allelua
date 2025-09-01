-- local table = require("table")
-- local coroutine = require("coroutine")
--
-- function coroutine.async_yield()
-- 	coroutine.yield(coroutine.parent)
-- end
--
-- local routines = {}
--
-- local Nursery = { _nursery = true, __metatable = false }
-- Nursery.__index = Nursery
--
-- function Nursery:go(routine)
-- 	local co = coroutine.create(routine)
-- 	self.ready[co] = co
-- end
--
-- function Nursery:_ready(co)
-- 	if self._pending[co] then
-- 		self._ready[co] = co
-- 		self._pending[co] = nil
-- 	end
-- end
--
-- function Nursery:_tick()
-- 	local parent = coroutine._parent
-- 	coroutine._parent = self
--
-- 	-- Resume all ready routines.
-- 	for co in pairs(self.ready) do
-- 		local ok, val = coroutine.resume(co, self)
--
-- 		-- Forward error if any.
-- 		if not ok then error(val) end
--
-- 		-- Move to pending set until completion move it back to ready set.
-- 		if type(val) == "table" and val._nursery == true then
-- 			self.ready[co] = nil
-- 			self.pending[co] = co
-- 		else
-- 			if coroutine.status(co) == "dead" then
-- 				self.ready[co] = nil -- remove coroutine
-- 			else
-- 				error("can't yield across nursery boundary")
-- 			end
-- 		end
-- 	end
--
-- 	coroutine._parent = parent
-- end
--
-- coroutine.nursery = function(block)
-- 	local co = coroutine.create(block)
-- 	local nu = setmetatable({ ready = { [co] = co }, pending = {} }, Nursery)
--
-- 	while true do
-- 		-- Executes all ready routines.
-- 		nu:_tick()
--
-- 		-- No more work to do.
-- 		if empty(nu.pending) then return end
--
-- 		-- Yield before next tick.
-- 		coroutine.yield(coroutine._parent)
-- 	end
-- end
--
-- function empty(t)
-- 	for _ in pairs(t) do return false end
-- 	return true
-- end
--
-- function main()
-- 	coroutine.nursery(function(n)
-- 		print("nursery 1", n)
--
-- 		n:go(function()
-- 			print("1.1")
-- 			coroutine.async_yield()
-- 			print("1.2")
-- 		end)
--
-- 		n:go(function()
-- 			print("2.1")
-- 			coroutine.async_yield()
-- 			print("2.2")
-- 			coroutine.async_yield()
-- 			print("2.3")
-- 		end)
--
-- 		n:go(function()
-- 			coroutine.nursery(function(n)
-- 				print("nursery 2", n)
--
-- 				n:go(function()
-- 					print("1.1")
-- 					coroutine.async_yield()
-- 					print("1.2")
-- 				end)
--
-- 				n:go(function()
-- 					print("2.1")
-- 					coroutine.async_yield()
-- 					print("2.2")
-- 					coroutine.async_yield()
-- 					print("2.3")
-- 				end)
--
-- 				n:go(function()
-- 					print("3.1")
-- 					coroutine.async_yield()
-- 					print("3.2")
-- 					coroutine.async_yield()
-- 					print("3.3")
-- 				end)
-- 			end)
-- 		end)
--
-- 		n:go(function()
-- 			print("3.1")
-- 			coroutine.async_yield()
-- 			print("3.2")
-- 			coroutine.async_yield()
-- 			print("3.3")
-- 		end)
-- 	end)
-- end
--
-- local m = coroutine.create(main)
-- while true do
-- 	local ok, res = coroutine.resume(m)
-- 	if not ok then error(res) end
--
-- 	if empty(routines) then break end
--
-- 	-- emulate zig loop
-- 	for co, n in pairs(routines) do
-- 		n:_ready(co)
-- 		routines[co] = nil
-- 	end
-- end
--

print("nursery...")
--print(sleep(0.5))
coroutine.nursery(function(n)
	print("block")
	n:go(function()
		print("sleep 1")
		sleep(1)
		print("done 1")
	end)

	n:go(function()
		print("sleep 2")
		sleep(2)
		print("done 2")
	end)

	n:go(function()
		coroutine.nursery(function(n)
			print("1 block")
			n:go(function()
				print("1 sleep 1")
				print(sleep(1))
				print("1 done 1")
			end)

			n:go(function()
				print("1 sleep 2")
				sleep(2)
				print("1 done 2")
			end)
		end)
	end)
end)
print("nursery done")
