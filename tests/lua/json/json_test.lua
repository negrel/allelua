local json = require("json")
local math = require("math")
local t = require("test")

t.test("encode sequence produce json array", function()
	local seq = { 1, true, false, "foo", { foo = true } }
	local seq_json = json.encode(seq)
	assert(seq_json == '[1,true,false,"foo",{"foo":true}]')
end)

t.test("encode sequence with nil produce json array", function()
	local seq = { 1, true, nil, "foo", { foo = true } }
	local seq_json = json.encode(seq)
	assert(seq_json == '[1,true,null,"foo",{"foo":true}]')
end)

t.test("pretty encode sequence with nil produce json array", function()
	local seq = { 1, true, nil, "foo", { foo = true } }
	local seq_json = json.encode(seq, { pretty = "true" })
	assert(seq_json == [==[[
  1,
  true,
  null,
  "foo",
  {
    "foo": true
  }
]]==])
end)

t.test("encode table produce json object", function()
	local tab =
		{ foo = true, bar = 1, huge = math.huge, nan = 0 / 0, null = json.null }
	local tab_json = json.encode(tab)

	assert(tab_json:contains('"foo":true'))
	assert(tab_json:contains('"bar":1'))
	assert(tab_json:contains('"huge":null'))
	assert(tab_json:contains('"nan":null'))
	assert(tab_json:contains('"null":null'))
end)

t.test("encode cyclic table returns a data error", function()
	local tab = {}
	local tab2 = { prev = tab }
	tab.next = tab2
	local ok, err = pcall(json.encode, tab)
	assert(
		not ok
			and err.kind == "Data"
			and err.message:contains("recursive table detected")
	)
end)

t.test("decode json array into sequence table", function()
	local json_str = '[1,false,"foo",{"foo":"foo"}]'
	local seq = json.decode(json_str)
	assert(table.deep_eq(seq, { 1, false, "foo", { foo = "foo" } }))
end)

t.test("decode json object into table", function()
	local json_str = '{"foo":"bar","null":null}'
	local tab = json.decode(json_str)
	assert(table.deep_eq(tab, { foo = "bar" }))
end)

t.test("decode invalid json returns syntax error", function()
	local json_str = "[1,false,'foo']" -- single quote for json strings is invalid
	local ok, err = pcall(json.decode, json_str)
	assert(
		not ok
			and err.kind == "Syntax"
			and err.message == "expected value at line 1 column 10"
	)
end)

t.test('decode invalid json with no closing "}" returns eof error', function()
	local json_str = '{"foo":"foo"' -- missing }
	local ok, err = pcall(json.decode, json_str)
	assert(
		not ok
			and err.kind == "Eof"
			and err.message == "EOF while parsing an object at line 1 column 12"
	)
end)
