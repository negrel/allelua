-- Run this file from root of repository
local mypkg = require("./mypkg")
assert(mypkg == require("@/examples/mypkg"))

print(mypkg)
print(mypkg.desc())
