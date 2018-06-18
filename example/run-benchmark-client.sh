cd benchmark-client
cargo build --release
./target/release/simple-bench test_lua_search
./target/release/simple-bench libtarantool_rust_api_example.test_bench
