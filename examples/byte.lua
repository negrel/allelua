local byte = require("byte")

local buf = byte.buffer(4, 66)
buf[1] = 65
buf[4] = 65
print("BUF1", buf)

local buf2 = byte.buffer_from_string("ABBA")
print("BUF2", buf2)
print("BUF1 == BUF2", buf == buf2)
