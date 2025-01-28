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
    * Lua is a lot simpler than other scripting language
    * Stable, based on Lua 5.1 with a few 5.2 compatibility features, core language
    won't change or will remains 100% compatible
* Easy concurrency:
    * No async/await
    * Write concurrent code like single threaded code using structured concurrency
* Directory based packages
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
    * Type checker (**wip**)
* FFI support (**planned**)

Our goal is to transform Lua, originally designed as an embeddable scripting
language, into a full-fledged, independent programming language capable of
standalone use.

## Examples

Here are a few examples:

### Structured concurrency

`allelua` supports structured concurrency built on top of Tokio. If you're
unfamiliar with structured concurrency and why unstructured concurrency isn't
supported read this article:
[Notes on structured concurrency, or: Go statement considered harmful](https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/)

```lua
coroutine.nursery(function(go)
    go(f, x, y, z)
    -- or
    go(function()
        f(x, y, z)
    end)
end)
```

starts a new coroutine running

```lua
f(x, y, z)
```

A coroutine is a lightweight thread managed by the `allelua` runtime. But unlike,
Go's goroutine, they can only be spawned in `coroutine.nursery()` function. Also
when `coroutine.nursery()` returns, all goroutines have finished to execute. This
prevents leaking routine.

Here is an example proving that goroutines runs concurrently:

```lua
local time = require("time")

coroutine.nursery(function(go)
    local now = time.Instant:now()
    for i = 1, 3 do
        go(function()
            time.sleep(1 * time.second)
            print("goroutine", i, "done in", now:elapsed())
        end)
    end
end)
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

### Packages

`allelua` supports directory based packages. If you have the following file
structure:

```sh
$ tree -d .
.
└── src/
    ├── mypkg/
    │   ├── foo.lua
    │   └── bar.lua
    └── main.lua
```

You can import `mypkg` package from main.lua:

```lua
-- import relative to current file
import "./mypkg"
-- import relative to current working directory
import "@/src/mypkg"

-- Print mypkg functions / variables.
print(mypkg)
```

Packages are entirely isolated and can't alter global environment (`_G`).

### Shell scripts

`allelua` is legacy-free and breaks compatibility with standard Lua. We provide our
own standard library with handy modules such as `sh` that provides a DSL for
shell scripting:

```shell
local os = require("os")

coroutine.nursery(function(go)
    local sh = require("sh").new(go)

    -- Pipe ls stdout into tr.
    -- ls -l | tr [a-z] [A-Z]
    local output = sh.ls("-l"):tr("[a-z]", "[A-Z]"):output()

    -- Pass io.Reader as stdin.
    -- You can also redirect stdout and stderr to io.Writer.
    local f = os.File.open("/path/to/file", { read = true })
    output = tr({ stdin = f }, "[a-z]", "[A-Z]"):output()
    print(output)
end)
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
