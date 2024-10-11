local package = require("package")
local os = require("os")
local sh = require("sh")

local f = os.File.open(package.meta.path, { read = true })

print(sh.tr("[a-z]", "[A-Z]"):tr("[A-Z]", "[a-z]"):output())
