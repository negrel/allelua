<h1 align="center">
    <img height="250" src="./.github/images/allelua.png">
</h1>

# 🙏 `allelua` - LuaJIT distribution blessed by programming gods

`allelua` is a Lua runtime with secure defaults and a great developer experience.
It's built on [`mlua`](https://github.com/mlua-rs/mlua),
[Rust](https://www.rust-lang.org/) and [Tokio](https://tokio.rs).

## Features

Here's what makes `allelua` a unique alternative to language like Python and
runtimes like Deno / NodeJS:

* Fast and resources efficient: LuaJIT is extremely fast and lightweight.
* Simple:
    * Lua is a lot simpler than other scripting language.
    * Stable, based on Lua 5.1 with a few 5.2 compatibility features
* Easy concurrency:
    * No async/await
    * Write concurrent code like single threaded code (using goroutines)
* Secure by default (**planned**)
* Batteries included:
    * Linter
    * Formatter
    * LSP
    * Package manager (**planned**)
    * Task runner (**planned**)
    * Test runner
    * Benchmarking tool
    * Documentation generator (**wip**)
* FFI support (**planned**)

Our goal is to transform Lua, originally designed as an embeddable scripting
language, into a full-fledged, independent programming language capable of
standalone use.

## Examples

Here are a few examples:

### Goroutine

A goroutine is a lightweight thread managed by the `allelua` runtime.

```lua
go(f, x, y, z)
-- or
go(function()
    f(x, y, z)
end)
```

starts a new goroutine running

```lua
f(x, y, z)
```

Here is an example proving that goroutines runs concurrently:

```lua
local time = require("time")

local now = time.Instant:now()
for i = 1, 3 do
	go(function()
		time.sleep(i * time.second)
		print("goroutine", i, "done in", now:elapsed())
	end)
end
```

prints

```lua
goroutine 1 done 1.001s
goroutine 2 done 1.001s
goroutine 3 done 1.001s
```

If goroutines were run one after another, they would print:

```lua
goroutine 1 done 1.001s
goroutine 2 done 2.001s
goroutine 3 done 3.001s
```

### Shell scripts

Allelua is legacy-free and breaks compatibility with standard Lua. We provide our
own standard library with handy modules such as `sh` that provides a DSL for
shell scripting:

```shell
local sh = require("sh")

-- Pipe ls stdout into tr.
-- ls -l | tr [a-z] [A-Z]
local output = sh.ls("-l"):tr("[a-z]", "[A-Z]"):output()
print(output)

-- Pass io.Reader as stdin.
-- You can also redirect stdout and stderr to io.Writer.
local f = os.File.open("/path/to/file", { read = true })
local output = tr({ stdin = f }, "[a-z]", "[A-Z]"):output()
print(output)
```

## Contributing

If you want to contribute to `allelua` to add a feature or improve the code contact
me at [alexandre@negrel.dev](mailto:alexandre@negrel.dev), open an
[issue](https://github.com/negrel/allelua/issues) or make a
[pull request](https://github.com/negrel/allelua/pulls).

## :stars: Show your support

Please give a :star: if this project helped you!

[![buy me a coffee](https://github.com/negrel/.github/raw/master/.github/images/bmc-button.png?raw=true)](https://www.buymeacoffee.com/negrel)

## :scroll: License

MIT © [Alexandre Negrel](https://www.negrel.dev/)
