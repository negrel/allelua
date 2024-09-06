use mlua::{chunk, Lua};

pub fn load_test(lua: &'static Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "test",
        lua.create_function(|lua, ()| {
            lua.load(chunk! {
                local debug = require("debug")
                local table = require("table")
                local os = require("os")
                local time = require("time")

                local M = {
                  __tests = {}
                }

                local real_print = print
                local test_print = function(file, name)
                  return function(...)
                    real_print(file .. " " .. name .. ": ", ...)
                  end
                end

                function M.test(name, test)
                  assert(type(name) == "string", "test name is not a string")
                  assert(type(test) == "function", "test body is not a function")
                  local info = debug.getinfo(2, "S")
                  local filename = info.short_src

                  local test_file_table = M.__tests[filename] or {}
                  table.insert(test_file_table, { name = name, test = test })

                  M.__tests[filename] = test_file_table
                end

                function M.__execute_suite()
                  local passed = 0
                  local failed = 0
                  local test_suite_result = "ok"
                  local start_instant = time.Instant.now()

                  // Run tests per source file.
                  for test_file, tests in pairs(M.__tests) do
                    print("running", #tests, "tests from", test_file)

                    // Run all tests from the same file
                    for _, test in ipairs(tests) do
                      local test_passed = M.__execute_test(test_file, test.name, test.test)
                      if test_passed then
                        passed = passed + 1
                      else
                        failed = failed + 1
                        test_suite_result = "FAILED" // one test failed
                      end
                    end
                  end

                  // Sum up results.
                  print("\n", test_suite_result, "|", passed, "passed |", failed, "failed |", start_instant:elapsed())

                  if failed > 0 then
                    os.exit(1)
                  end
                end

                function M.__execute_test(file, name, test)
                    local start_instant = time.Instant.now()
                    print = test_print(file, name) // replace std print

                    local success, error_msg = pcall(test)

                    print = real_print // restore std print

                    local test_duration = start_instant:elapsed()

                    // Print result.
                    if success then
                        print(name, "...", "ok", test_duration)
                    else
                        print(name, " ... ", "FAILED", elapsed)
                        print(debug.traceback(error_msg))
                        print()
                    end

                    return success
                    end

                function M.assert_eq(left, right, msg)
                  if not table.deep_eq(left, right) then
                    real_print("values are not equal")
                    real_print("left  :", left)
                    real_print("right :", right)
                    error(msg)
                  end
                end

                return M
            })
            .eval::<mlua::Table>()
        })?,
    )
}
