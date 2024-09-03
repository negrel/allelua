local byte = require("byte")

local buf = byte.Buffer.new(4, 66)
buf[1] = 65
buf[4] = 65
print("BUF1", buf)

local buf2 = byte.Buffer.from_string("ABBA")
print("BUF2", buf2)
print("BUF1 == BUF2", buf == buf2)
print("BUF1 is same BUF2", rawequal(buf, buf2))
