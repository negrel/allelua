local env = require("env")

print("current working directory:", env.current_dir())
print("env vars PWD:", env.vars["PWD"])
print("args:", env.args)
