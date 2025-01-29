return function(M, worker_new, is_worker, worker_input, worker_output)
	local sync = require("sync")
	local buffer = require("string.buffer")
	local package = require("package")
	local coroutine = require("coroutine")

	M.Worker = { __type = "proc.Worker" }
	M.Worker.__index = M.Worker

	M.Worker.new = function(fpath)
		local w = { _internal = worker_new(package.resolve_path(fpath, 2)) }
		w._input = sync.Mutex.new(w._internal:input())

		local output = w._internal:output()
		local buf = buffer.new()
		local read = output:read(buf, 1)
		if read == 0 then error("failed to spawn worker process") end

		w._output = sync.Mutex.new(output)

		return setmetatable(w, M.Worker)
	end

	function M.Worker:post(msg)
		local emsg = buffer.encode(msg)
		local input = self._input:lock()
		input:write_string(buffer.encode(#emsg))
		input:write_string(emsg)

		self._input:unlock()
	end

	function M.Worker:on_message(handler)
		coroutine.nursery(function(go)
			local MSGSIZE_LEN = 9
			local sizebuf = buffer.new(MSGSIZE_LEN)
			local msgbuf = buffer.new()

			local output = self._output:lock()

			while true do
				-- Read size of message.
				local read = output:read(sizebuf, MSGSIZE_LEN)
				if read == 0 then break end
				local msgsize = sizebuf:decode()

				-- Read message.
				msgbuf:reserve(msgsize)
				read = output:read(msgbuf, msgsize)
				assert(
					read == msgsize,
					("received message size (%d) doesn't match announced size (%d)"):format(
						read,
						msgsize
					)
				)
				local msg = msgbuf:decode()

				-- Handle message
				go(handler, msg)

				sizebuf:reset()
				msgbuf:reset()
			end

			self._output:unlock()
		end)
	end

	function M.Worker:terminate()
		self._internal:terminate()
	end

	if is_worker then
		M.parent = {
			__type = "proc.Parent",
			-- TODO: inject input/output
			_input = sync.Mutex.new(worker_input),
			_output = sync.Mutex.new(worker_output),
		}

		function M.parent:on_message(handler)
			coroutine.nursery(function(go)
				local MSGSIZE_LEN = 9
				local sizebuf = buffer.new(MSGSIZE_LEN)
				local msgbuf = buffer.new()

				local input = self._input:lock()

				while true do
					-- Read size of message.
					local read = input:read(sizebuf, MSGSIZE_LEN)
					assert(read == MSGSIZE_LEN)
					local msgsize = sizebuf:decode()

					-- Read message.
					msgbuf:reserve(msgsize)
					read = input:read(msgbuf, msgsize)
					assert(
						read == msgsize,
						("received message size (%d) doesn't match announced size (%d)"):format(
							read,
							msgsize
						)
					)
					local msg = msgbuf:decode()

					-- Handle message
					go(handler, msg)

					sizebuf:reset()
					msgbuf:reset()
				end

				self._input:unlock()
			end)
		end

		function M.parent:post(msg)
			local emsg = buffer.encode(msg)
			local output = self._output:lock()
			output:write_string(buffer.encode(#emsg))
			output:write_string(emsg)

			self._output:unlock()
		end
	end
end
