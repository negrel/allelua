#!/usr/bin/env bats

: ${ALLELUA:="allelua"}

bats_require_minimum_version 1.5.0

setup() {
	# get the containing directory of this file
	# use $BATS_TEST_FILENAME instead of ${BASH_SOURCE[0]} or $0,
	# as those will point to the bats executable's location or the preprocessed file respectively
	DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")" >/dev/null 2>&1 && pwd )"
}

@test "allelua test fail_test.lua" {
	run -1 $ALLELUA test "$DIR"/data/fail_test.lua
	grep "oops" <<< "$output"
	grep "test that fail ... FAILED" <<< "$output"
	grep "FAILED | 0 passed | 1 failed |" <<< "$output"
}

@test "allelua test success_test.lua" {
	run -0 $ALLELUA test "$DIR"/data/success_test.lua
	grep "test that succeed ... ok" <<< "$output"
}

@test "allelua test notfound_test.lua" {
	run -1 $ALLELUA test "$DIR"/data/notfound_test.lua
	grep "notfound_test.lua: No such file or directory (os error 2)" <<< "$output"
}

@test "allelua test dir/" {
	run -1 $ALLELUA test "$DIR"
	grep "oops" <<< "$output"
	grep "test that fail ... FAILED" <<< "$output"
	grep "FAILED | 0 passed | 1 failed |" <<< "$output"
	grep "test that succeed ... ok" <<< "$output"
}
