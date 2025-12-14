--- Start executing user main module/function.
function _rs.start(chunk)
	local env = setmetatable({}, { __index = _G })
	setfenv(chunk, env)

	-- Execute chunk.
	chunk()

	if type(env.main) == "function" then
		env.main(_rs.args)
	end
end
