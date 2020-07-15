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

    box.schema.func.create('libtarantool_rust_api_example.test_bench', {language = 'C'})
    box.schema.user.grant('guest', 'execute', 'function', 'libtarantool_rust_api_example.test_bench')
end
bootstrap()
local json = require('json')

local function init_load_spaces()
    if(box.space.countries ~=nil) then
        box.space.countries:drop();
    end
    local countriesSpace = box.schema.create_space('countries', {id = 1001})
    countriesSpace:create_index('primary', {type='tree', parts={1,'unsigned'}})
    local file = io.open("countries.json", "rb")
    local content = file:read "*a"
    file:close()
    local currencies = json.decode(content);
    for i,row in pairs(currencies) do
        countriesSpace:insert({
            row["country-code"],
            string.lower(row["name"]),
            string.lower(row["region"]),
            string.lower(row["sub-region"]),
        })
    end
end
init_load_spaces()

local msgpack = require('msgpack')
net_box = require('net.box')
capi_connection = net_box:new(3301)

local function check_attr(search_str, row, index )
    return  search_str==nil or string.find(row[index],  search_str);
end
function test_lua_search(country_name, region, sub_region)
    local result = {};
    local country_name_l = country_name ~= nil and string.lower(country_name) or nil;
    local region_l = region~=nil and string.lower(region) or nil;
    local sub_region_l = sub_region ~=nil and string.lower(sub_region) or nil;
    for i,row in box.space.countries.index.primary:pairs() do
        if( check_attr(country_name_l, row, 2) and
                check_attr(region_l, row,3) and
                check_attr(sub_region_l,row,4) ) then
            table.insert(result,{
                ["country-code"]=row[1],
                ["name"]=row[2],
                ["region"]=row[3],
                ["sub-region"]=row[4],
            })
--            table.insert(result, row)
        end
    end
    return {result};
end

local ffi = require('ffi')
ffi.cdef[[
        void init_dictionaries_ffi();
    ]]
rust = ffi.load('./libtarantool_rust_api_example.so')
rust.init_dictionaries_ffi();
local refresh_dict_fn = function() rust.init_dictionaries_ffi(); end;
box.space._space:on_replace(refresh_dict_fn);
box.space._index:on_replace(refresh_dict_fn);

print("call rust !",json.encode(capi_connection:call('libtarantool_rust_api_example.test_bench', {'RU','EUR', msgpack.NULL})))
print("call lua!",json.encode(capi_connection:call('test_lua_search', {'RU','EUR', msgpack.NULL})))

--os.exit();