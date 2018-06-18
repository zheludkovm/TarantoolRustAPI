cargo build
cp target/debug/libtarantool_rust_api_example.so test-tarantool/
cd test-tarantool
./clean-db.sh
tarantool ffi-tests.lua
./clean-db.sh
tarantool functional-tests.lua