#!        /usr/bin/tarantool
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
    
    too_long_threshold = 0.5;
    background = false;
    pid_file = 'rust.pid';
}

local ffi = require('ffi')
ffi.cdef[[
        void init_dictionaries_ffi();
    ]]
rust = ffi.load('./libtarantool_rust_api_example.so')
rust.init_dictionaries_ffi();
local refresh_dict_fn = function() rust.init_dictionaries_ffi(); end;
box.space._space:on_replace(refresh_dict_fn);
box.space._index:on_replace(refresh_dict_fn);

local function grantRightsToFunction(fnName)
    box.schema.func.create(fnName, { language = 'C' })
    box.schema.user.grant('guest', 'execute', 'function', fnName)
end

local function bootstrap()
    box.schema.user.grant('guest', 'read,write,execute', 'universe')
    box.schema.user.create('rust', { password = 'rust' })
    box.schema.user.grant('rust', 'read,write,execute', 'universe')

    grantRightsToFunction('libtarantool_rust_api_example.test_insert');
    grantRightsToFunction('libtarantool_rust_api_example.test_replace');
    grantRightsToFunction('libtarantool_rust_api_example.test_index_get');
    grantRightsToFunction('libtarantool_rust_api_example.test_index_get_raw');
    grantRightsToFunction('libtarantool_rust_api_example.test_delete');
    grantRightsToFunction('libtarantool_rust_api_example.test_update');
    grantRightsToFunction('libtarantool_rust_api_example.test_upsert');
    grantRightsToFunction('libtarantool_rust_api_example.test_iterator');
    grantRightsToFunction('libtarantool_rust_api_example.test_min_max_count');
    grantRightsToFunction('libtarantool_rust_api_example.test_truncate');
    grantRightsToFunction('libtarantool_rust_api_example.test_lua_call');
    grantRightsToFunction('libtarantool_rust_api_example.test_get_space_id');
end

box.once('grants3', bootstrap)


local function init_test_spaces()
    if (box.space.test_space ~= nil) then
        box.space.test_space:drop();
    end
    box.schema.create_space('test_space', { engine = 'memtx' })
    box.space.test_space:create_index('primary', { type = 'tree', parts = { 1, 'number' } })
    box.space.test_space:create_index('secondary', { type = 'tree', parts = { 2, 'string' } })
    if (box.sequence.test_seq ~= nil) then
        box.sequence.test_seq:drop()
    end
    box.schema.sequence.create('test_seq', { min = 5, start = 5 })
end

local net_box = require('net.box')
local tap = require('tap')
local json = require('json')
local msgpack = require('msgpack')

local capi_connection = net_box:new(3301)



local testPlan = tap.test("test plan")
testPlan:plan(11)
testPlan:test("insert test", function(test)
    test:plan(3)
    init_test_spaces()
    local res = capi_connection:call('libtarantool_rust_api_example.test_insert', { 1, "test insert", { a = 1, b = "b" } })
    test:is(res[1], true, "call insert is ok")
    test:is(box.space.test_space:get(1)[2], "test insert", "insert value is ok")
    test:is(box.space.test_space:get(1)[3].a, 1, "insert struct is ok")
end)
testPlan:test("replace test", function(test)
    test:plan(2)
    init_test_spaces()
    box.space.test_space:put({ 1, 'row to replace', nil })
    local res = capi_connection:call('libtarantool_rust_api_example.test_replace', { 1, "replaced", { a = 2, b = "c" } })
    test:is(res[1], true, "call replace is ok")
    test:is_deeply(box.space.test_space:get(1):totable(), { 1, "replaced", { a = 2, b = "c" } }, "replace value is ok")
end)
testPlan:test("index get test", function(test)
    test:plan(2)
    init_test_spaces()
    box.space.test_space:put({ 1, 'test-row', { a = 1, b = "b" } })
    local row = capi_connection:call('libtarantool_rust_api_example.test_index_get', { 1 })
    test:is_deeply(row[1], { 1, 'test-row', { a = 1, b = "b" } }, "get value is ok")
    local row = capi_connection:call('libtarantool_rust_api_example.test_index_get_raw', { 1 })
    test:is_deeply(row[1], { 1, 'test-row', { a = 1, b = "b" } }, "get raw value is ok")
end)
testPlan:test("delete test", function(test)
    init_test_spaces()
    box.space.test_space:put({ 1, 'test-row', { a = 1, b = "b" } })
    local res = capi_connection:call('libtarantool_rust_api_example.test_delete', { 1 })

    test:plan(2)
    test:is(res[1], true, "call is")
    test:is(box.space.test_space:get(1), nil, "value deleted ok")
end)
testPlan:test("update test", function(test)
    init_test_spaces()
    box.space.test_space:put({ 1, 'test-row', { a = 1, b = "b" } })
    local res = capi_connection:call('libtarantool_rust_api_example.test_update', { { 1 }, 3, { a = 2, b = "c" } })

    test:plan(2)
    test:is(res[1], true, "call is ok")
    test:is_deeply(box.space.test_space:get(1)[3], { a = 2, b = "c" }, "value updated ok")
end)
testPlan:test("upsert test", function(test)
    init_test_spaces()
    local res = capi_connection:call('libtarantool_rust_api_example.test_upsert', { { 1, 'test-row', { a = 1, b = "b" } }, 3, { a = 1, b = "b" } })

    test:plan(4)
    test:is(res[1], true, "call1 is ok")
    test:is_deeply(box.space.test_space:get(1)[3], { a = 1, b = "b" }, "value upserted ok-insert")

    local res = capi_connection:call('libtarantool_rust_api_example.test_upsert', { { 1, 'test-row-updated', { a = 1, b = "b" } }, 3, { a = 2, b = "c" } })

    test:is(res[1], true, "call2 is ok")
    test:is_deeply(box.space.test_space:get(1):totable(), { 1, 'test-row', { a = 2, b = "c" } }, "value upserted ok-update only one field")
end)
testPlan:test("iterator test", function(test)
    init_test_spaces()
    box.space.test_space:put({ 1, 'test-1row', { a = 1, b = "b" } })
    box.space.test_space:put({ 2, 'test-2row', { a = 1, b = "b" } })
    box.space.test_space:put({ 3, 'test-3row', { a = 1, b = "b" } })
    box.space.test_space:put({ 5, 'test-5row', { a = 1, b = "b" } })
    box.space.test_space:put({ 6, 'test-51row', { a = 1, b = "b" } })
    box.space.test_space:put({ 7, 'test-52row', { a = 1, b = "b" } })

    test:plan(2)
    local res = capi_connection:call('libtarantool_rust_api_example.test_iterator', { 2, "not exist" })
    test:is(res[1], msgpack.NULL, "value not exist")
    local res = capi_connection:call('libtarantool_rust_api_example.test_iterator', { 6, ".*5.*" })
    test:is_deeply(res[1], { { 6, 'test-51row', { a = 1, b = "b" } },{ 7, 'test-52row', { a = 1, b = "b" } }}, "value found ok")
end)
testPlan:test("index min max count test", function(test)
    init_test_spaces()
    box.space.test_space:put({ 1, 'test-1row', { a = 1, b = "b" } })
    box.space.test_space:put({ 2, 'test-2row', { a = 1, b = "b" } })
    box.space.test_space:put({ 3, 'test-3row', { a = 1, b = "b" } })
    box.space.test_space:put({ 4, 'test-5row', { a = 1, b = "b" } })

    test:plan(3)
    local res = capi_connection:call('libtarantool_rust_api_example.test_min_max_count', { "test-3" })[1]
    test:is(res[1][1], 3, "min value ok, row 3 - min row with index greater then test-3")
    test:is(res[2][1], 2, "max value ok, row 2 - max row with index lower then test-3")
    test:is(res[3], 2, "count value ok - two rows greater or equal test-3")
end)
testPlan:test("truncate test", function(test)
    init_test_spaces()
    box.space.test_space:put({ 1, 'test-1row', { a = 1, b = "b" } })
    box.space.test_space:put({ 2, 'test-2row', { a = 1, b = "b" } })

    test:plan(2)
    local res = capi_connection:call('libtarantool_rust_api_example.test_truncate', {})
    test:is(res[1], true, "call is ok")
    test:is(box.space.test_space:count(), 0, "space truncated")
end)
function test_fn(zero_value, num_value, str_value, tuple_value)
--    print("call fn! tuple_value", json.encode(tuple_value));
    local result_table = tuple_value:totable();
    result_table[1] = result_table[1] + 1;
    result_table[2] = result_table[2] .. '_sufix';
    result_table[3].a = result_table[3].a+1;
    result_table[3].b = result_table[3].b .. '_sufix';

    return zero_value, zero_value == nil, num_value + 1, str_value .. '_sufix', box.tuple.new(result_table);
end

testPlan:test("call lua function test", function(test)
    init_test_spaces()

    test:plan(5)
    local res = capi_connection:call('libtarantool_rust_api_example.test_lua_call', { nil, 2, "str", { 1, "str", {a=2, b="b"} } })[1]
    test:is_deeply(res[1],  {2, 'str_sufix', {a=3, b="b_sufix"}}, "return tuple is ok")
    test:is(res[2],  'str_sufix', "return string is ok")
    test:is(res[3],  3, "return number is ok")
    test:is(res[4],  true, "return bool is ok")
    test:is(res[5],  nil, "return nil is ok")
end)

testPlan:test("call sync dict test", function(test)
    init_test_spaces();

    test:plan(2)
    local res = capi_connection:call('libtarantool_rust_api_example.test_get_space_id', { "test_space" })
    test:is(res[1],  box.space._space.index.name:get("test_space")[1], "space id is ok")
    init_test_spaces();
    local res = capi_connection:call('libtarantool_rust_api_example.test_get_space_id', { "test_space" })
    test:is(res[1],  box.space._space.index.name:get("test_space")[1], "space id is ok")
end)


testPlan:check()

os.exit();