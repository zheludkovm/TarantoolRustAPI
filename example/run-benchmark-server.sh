cargo build  --release
cp target/release/libtarantool_rust_api_example.so test-tarantool/
cd test-tarantool
./clean-db.sh
tarantool init-benchmark-server.lua