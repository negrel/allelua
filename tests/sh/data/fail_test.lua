local t = require("test")

t.test("test that fail", function() error("oops") end)
