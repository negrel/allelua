local table = require("table")

function table.pack(...) return { ..., n = select('#', ...) } end

