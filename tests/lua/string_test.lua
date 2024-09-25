local string = require("string")
local t = require("test")

t.test(
	"string.contains('allelua', 'lua') returns true",
	function() assert(string.contains("allelua", "lua")) end
)

t.test(
	"string.contains('allelua', 'all') returns true",
	function() assert(string.contains("allelua", "all")) end
)

t.test(
	"string.contains('allelua', 'lel') returns true",
	function() assert(string.contains("allelua", "lel")) end
)

t.test(
	"string.contains('allelua', '') returns true",
	function() assert(string.contains("allelua", "")) end
)

t.test(
	"string.contains('allelua', 'allelua') returns true",
	function() assert(string.contains("allelua", "allelua")) end
)

t.test(
	"string.contains('allelua', 'ALLELUA') returns false",
	function() assert(not string.contains("allelua", "ALLELUA")) end
)

t.test(
	"string.contains('allelua', 'alleluaa') returns false",
	function() assert(not string.contains("allelua", "alleluaa")) end
)

t.test(
	"string.contains('allelua', 'Lua') returns false",
	function() assert(not string.contains("allelua", "Lua")) end
)
