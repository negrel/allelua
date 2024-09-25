use mlua::{chunk, Lua};

pub fn load_test(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "test",
        lua.create_function(|lua, ()| {
            lua.load(chunk! {
                local debug = require("debug")
                local table = require("table")
                local sync = require("sync")
                local os = require("os")
                local time = require("time")
                local path = require("path")
                local package = require("package")
                local jit = require("jit")

                local M = {
                    __tests = {},
                    __benchs = {}
                }

                local real_print = print
                local test_print = function(file, name)
                    name = name .. ":"
                    return function(...)
                        real_print(file, name, ...)
                    end
                end

                local assert_called_from_test_file = function()
                    local fpath = package.meta.path
                    if not path.file_stem(fpath):has_suffix("_test") then
                        error("this function must be called from a *_test.lua file", 2)
                    end
                end

                local assert_called_from_bench_file = function()
                    local fpath = package.meta.path
                    if not path.file_stem(fpath):has_suffix("_bench") then
                        error("this function must be called from a *_bench.lua file", 2)
                    end
                end

                function M.test(name, test, opts)
                    assert_called_from_test_file()
                    assert(type(name) == "string", "test name is not a string")
                    assert(type(test) == "function", "test body is not a function")
                    local filename = package.meta.path

                    local test_file_table = M.__tests[filename] or {}
                    table.insert(test_file_table, { name = name, test = test, opts = opts })

                    M.__tests[filename] = test_file_table
                end

                function M.__execute_test_suite()
                    local passed = 0
                    local failed = 0
                    local test_suite_result = "ok"
                    local start_instant = time.Instant.now()

                    // Run tests per source file.
                    for test_file, tests in pairs(M.__tests) do
                        print("running", #tests, "tests from", test_file)

                        // Run all tests from the same file
                        for _, test in ipairs(tests) do
                            local opts = test.opts or {}
                            if not opts.timeout then opts.timeout = 5 * time.second end
                            local test_passed = M.__execute_test(test_file, test.name, test.test, opts)
                            if test_passed then
                                passed = passed + 1
                            else
                                failed = failed + 1
                                test_suite_result = "FAILED" // one test failed
                            end
                        end
                    end

                    // Sum up results.
                    print(test_suite_result, "|", passed, "passed |", failed, "failed |", start_instant:elapsed(), "\n")
                    return failed == 0
                end

                function M.__execute_test(file, name, test, opts)
                    local start_instant = time.Instant.now()
                    print = test_print(file, name) // replace std print

                    local tx, rx = sync.channel()
                    local abort_test = go(function()
                        local results = { pcall(test) }
                        tx:send(results)
                        tx:close()
                    end)

                    local success, error_msg
                    local timeout, abort_timeout = time.after(opts.timeout)
                    select {
                        [timeout] = function()
                            abort_test()
                            success = false
                            error_msg = "test timed out"
                        end,
                        [rx] = function(result)
                            abort_timeout()
                            success = result[1]
                            error_msg = result[2]
                        end,
                    }
                    print = real_print // restore std print

                    local test_duration = start_instant:elapsed()

                    // Print result.
                    if success then
                        print("\t", name, "...", "ok", test_duration)
                    else
                        print("\t", name, "...", "FAILED", test_duration)
                        if error_msg then
                            print(debug.traceback(error_msg))
                        end
                        print()
                    end

                    return success
                end

                function M.bench(name, bench, opts)
                    assert_called_from_bench_file()
                    assert(type(name) == "string", "benchmark name is not a string")
                    assert(type(bench) == "function", "benchmark body is not a function")

                    local filename = package.meta.path

                    local bench_file_table = M.__benchs[filename] or {}
                    table.insert(bench_file_table, { name = name, bench = bench, opts = opts })

                    M.__benchs[filename] = bench_file_table
                end

                function M.__execute_bench_suite()
                    local error = 0
                    local bench_suite_result = "ok"
                    local start_instant = time.Instant.now()

                    // Run benchs per source file.
                    for bench_file, benchs in pairs(M.__benchs) do
                        print("running", #benchs, "benchmarks from", bench_file)

                        // Run all tests from the same file
                        for _, bench in ipairs(benchs) do
                            local opts = bench.opts or {}
                            opts.bench_time = opts.bench_time or time.second
                            local bench_ok = M.__execute_bench(bench_file, bench.name, bench.bench, opts)
                            if not bench_ok then
                                error = error + 1
                                bench_suite_result = "FAILED"
                            end
                        end
                    end
                    // Sum up results.
                    print(bench_suite_result, "|", error, "error |", start_instant:elapsed(), "\n")

                    return error == 0
                end

                local run_n = function(n, bench)
                    // Try to get a comparable environment for each run
                    // by clearing garbage from previous runs.
                    collectgarbage()
                    collectgarbage()
                    local b = { n = n }

                    // Execute bench.
                    local success, error_msg = pcall(bench, b)
                    if not success then
                        print("\t", name, "...", "FAILED")
                        if error_msg then
                            print(debug.traceback(error_msg))
                        end
                        print()
                    end

                    return success
                end

                function M.__execute_bench(file, name, bench, opts)
                    jit.on(bench, true)

                    local n = 1
                    local dur = 0 * time.second
                    while dur < opts.bench_time do
                        local now = time.Instant.now()

                        local success = run_n(n, bench)
                        if not success then
                            print("\t", name, "...", "FAILED")
                            if error_msg then
                                print(debug.traceback(error_msg))
                            end
                            print()
                            return false
                        end

                        dur = now:elapsed()
                        n = n * 2
                    end

                    print("\t", name, "...", n, "iter", dur / n, "/ op")

                    return true
                end

                M.assert = assert

                function M.assert_eq(left, right, msg)
                    if not table.deep_eq(left, right) then
                        real_print("values are not equal")
                        real_print("left  :", left)
                        real_print("right :", right)
                        error(msg)
                    end
                end

                function M.assert_err(func, expected_err)
                    local ok, err = pcall(func)
                    M.assert_eq(err, expected_err, "error doesn't match expected error")
                end

                return M
            })
            .eval::<mlua::Table>()
        })?,
    )
}
