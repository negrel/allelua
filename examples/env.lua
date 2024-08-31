local env = require("env")

print("current working directory:", env.current_dir())
print("env vars:", env.vars())
print("args:", env.args())
