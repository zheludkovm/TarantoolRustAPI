#! /usr/bin/tarantool
box.cfg {
    listen = 3301;
    io_collect_interval = nil;
    readahead = 16320;
    memtx_memory = 4 * 1024 * 1024 * 1024;
    memtx_min_tuple_size = 16;
    memtx_max_tuple_size = 10 * 1024 * 1024; 
    vinyl_max_tuple_size = 10 * 1024 * 1024; 
    vinyl_memory = 10 * 1024 * 1024 * 1024; 
    vinyl_cache = 1024 * 1024 * 1024; 
    vinyl_write_threads = 2;
    wal_mode = "write";
    wal_max_size = 256 * 1024 * 1024;
    checkpoint_interval = 60 * 60; 
    checkpoint_count = 6;
    snap_io_rate_limit = nil;
    force_recovery = true;
    log_level = 5;
    log = "./tarantool.log",
    wal_dir = './db/wal',
    memtx_dir = './db/memtx',
    vinyl_dir = './db/vinyl',
    --log_nonblock = true;
    too_long_threshold = 0.5;
    background = false;
    pid_file = 'rust.pid';
}

local function bootstrap()
    box.schema.user.grant('guest', 'read,write,execute', 'universe')
    box.schema.user.create('rust', { password = 'rust' })
    box.schema.user.grant('rust', 'read,write,execute', 'universe')
end
box.once('grants', bootstrap)


local ffi = require('ffi')
ffi.cdef[[
    int test_ffi(int v, const char *data);
]]
rust = ffi.load('./libtarantool_rust_api_example.so')

local tap = require('tap')
test = tap.test("test plan")
test:plan(1)
test:test("ffi tests", function(test)
    test:plan(1)
    local result = rust.test_ffi(10, "string");
    test:is(result, 11, "test ffi function is ok")
end)
test:check()

os.exit();