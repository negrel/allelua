return function()
	local string = require("string")
	local table = require("table")
	local os = require("os")
	local io = require("io")

	local M = {}

	local function copy_then_close(src, dst)
		local copied = io.copy(src, dst)
		src:close()
		dst:close()
		return copied
	end

	M.Error = { __type = "sh.CommandError" }

	function M.Error:new(cmd, proc, status)
		local err = { cmd = cmd, proc = proc, status = status }
		setmetatable(err, self)
		self.__index = self
		return err
	end

	function M.Error:__tostring()
		local str = "process "
			.. string.format("%q", tostring(self.cmd))
			.. " failed"
		if self.status.code then
			str = str .. " and exited with status code " .. tostring(self.status.code)
		end
		str = str .. "."

		return str
	end

	local Command = { __type = "sh.Command" }

	function Command:__index(k)
		-- Looking for property.
		if rawget(Command, k) then return rawget(Command, k) end

		-- Private property.
		if k:has_prefix("_") then return nil end

		-- This command is piped into another command.
		local cmd = Command:_new(k, self._sh)
		return self:pipe(cmd)
	end

	function Command:_new(name, sh)
		local cmd = { _name = name, _args = {}, _sh = sh }
		setmetatable(cmd, self)
		return cmd
	end

	function Command:__call(...)
		local args = { ... }

		-- removes args[1] if it is a Command. This happen when you do cmd1(...):cmd2(...)
		if type(args[1]) == "sh.Command" then table.shift(args) end

		self:_prepare_args(args)

		return self
	end

	function Command:_prepare_args(args)
		for k, v in pairs(args) do
			if rawtype(k) ~= "number" then
				if k == "stdin" then
					self._stdin = v
				elseif k == "stdout" then
					self._stdout = v
				elseif k == "stderr" then
					self._stderr = v
				else
					k = tostring(k)
					if #k == 1 then
						table.push(self._args, "-" .. k)
					else
						table.push(self._args, "--" .. k)
					end
				end
			else
				if rawtype(v) == "table" then
					self:_prepare_args(v)
				elseif rawtype(v) ~= "boolean" then
					table.push(self._args, tostring(v))
				end
			end
		end
	end

	function Command:__tostring()
		local quoted_args = table.map_values(self._args, string.quote)
		local cmd = self._name .. " " .. table.concat(quoted_args, " ")
		if self._stdin then return tostring(self._stdin) .. " | " .. cmd end
		return cmd
	end

	function Command:exec()
		-- Process is already running.
		if self._proc then return self._proc end

		local stdin = "inherit"
		if type(self._stdin) == "sh.Command" then
			stdin = "piped"
		elseif rawtype(self._stdin) == "string" then
			stdin = self._stdin
		else
			stdin = "piped"
		end

		local stdout = self._stdout or "piped"
		if type(self._stdout) == "sh.Command" then
			stdout = self._stdout._proc.stdin
		end

		local stderr = self._stderr or "piped"
		if type(self._stderr) == "sh.Command" then
			-- stderr and stdout are redirected to the same stream.
			-- Rust stdlib Command doesn't support this so we use a piped stderr
			-- and we manually copy stderr to process stdin.
			if self._stderr._proc.stdin == stdout then
				self._stdout = stdout
				self._stderr = stdout
				stdout = "piped"
				stderr = "piped"
			else
				stderr = self._stderr._proc.stdin
			end
		end

		self._proc = os.exec(
			self._name,
			{ args = self._args, stdin = stdin, stdout = stdout, stderr = stderr }
		)

		-- Execute input process.
		if type(self._stdin) == "sh.Command" and self._proc.stdin then
			self._stdin:exec()
		elseif
			self._stdin
			and type(self._stdin) ~= "sh.Command"
			and stdin == "piped"
		then
			self._sh.go(copy_then_close, self._stdin, self._proc.stdin)
		end

		-- copy stdout to configured writer.
		if
			self._stdout
			and type(self._stdout) ~= "sh.Command"
			and stdout == "piped"
		then
			self._sh.go(copy_then_close, self._proc.stdout, self._stdout)
		end

		-- copy stderr to configured writer.
		if
			self._stderr
			and type(self._stderr) ~= "sh.Command"
			and stderr == "piped"
		then
			self._sh.go(copy_then_close, self._proc.stderr, self._stderr)
		end

		return self._proc
	end

	-- Retrieves first command of a pipe chain.
	function Command:_pipe_head()
		if self._stdin then return self._stdin:_pipe_head() end
		return self
	end

	-- Retrieves last command of a pipe chain.
	function Command:_pipe_tail()
		if self._stdout then return self._stdout:_pipe_tail() end
		if self._stderr then return self._stderr:_pipe_tail() end
		return self
	end

	function Command:pipe(cmd)
		cmd = cmd:_pipe_head()
		cmd._stdin = self
		self._stdout = cmd
		return cmd:_pipe_tail()
	end

	function Command:pipe_err(cmd)
		cmd = cmd:_pipe_head()
		cmd._stdin = self
		self._stderr = cmd
		return cmd:_pipe_tail()
	end

	function Command:pipe_all(cmd)
		cmd = cmd:_pipe_head()
		cmd._stdin = self
		self._stdout = cmd
		self._stderr = cmd
		return cmd:_pipe_tail()
	end

	function Command:output(opts)
		opts = opts or {}
		opts.ignore_error = opts.ignore_error or false

		local out = nil

		local proc = self:exec()
		if proc.stdout then out = proc.stdout:read_to_end() end

		local status = proc:wait()
		if status.success or opts.ignore_error then return out end

		error(M.Error:new(self, proc, status))
	end

	function Command:error()
		local err = nil

		local proc = self:exec()
		if proc.stderr then err = proc.stderr:read_to_end() end

		local status = proc:wait()
		if not status.success then return err end

		return nil
	end

	function M.new(go)
		assert(rawtype(go) == "function", "go isn't a function")
		local sh = { go = go }
		setmetatable(sh, {
			__index = function(t, k)
				if M[k] then return M[k] end
				return Command:_new(k, sh)
			end,
		})
		return sh
	end

	return M
end
