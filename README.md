# Rust API to write tarantool stored procedures  

Rust wrapper over tarantool C API 

https://tarantool.io/en/doc/2.0/dev_guide/reference_capi/index.html  

## Overview

Tarantool is modern, fast NoSQL DB.

It provides two APIs to write stored procedures : LUA and C 

Lua API is very convenient, but it lacks performance in some scenarios like complex string processing. 

C API is too complex to write some trivial processing like search in tables.

This project intended to provide simple safe RUST API over tarantool C API.

You can write stored procedures which manipulate tarantool data, call lua code etc.

for details of the API - please look at example folder. There you can find functional tests for all api
and simple benchmark. 

### Some notes on serialization and deserialization :

Internally tarantool uses msgpack format (effective binary format to store tree structures)

In this project we use serde framework and serde msgpack backend https://github.com/3Hren/msgpack-rust

Serde by default serialize struct as sequence, please note this.

Row and keys in tarantool are tuples, in msgpack format they are stored as sequence.

So you can simply use rust structs or rust tuples as alter-ego of keys and rows.

in process of deserialization serde can detect map and also deserialize it as rust struct - 
for embedded lua tables in tarantool fields you also can use rust structs.

if you need serialize rust struct as map you can implement Serialize for struct and write code like this

```rust
impl Serialize for TestStruct {
    fn serialize< S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("a", &self.a)?;
        map.serialize_entry("b", &self.b)?;
        map.end()
    }
}
```


## Example RUST Stored procedure

Let's write simple stored procedure to search row in table by regexp


### Rust code
```rust
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate tarantool_rust_api;

use regex::Regex;
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::vec::Vec;
use tarantool_rust_api::tarantool::api::*;

//tarantool space name
static TEST_SPACE: &str = "test_space";
//tarantool index in space
static PRIMARY_INDEX: &str = "primary";

// embedded struct in tarantool row 
#[derive(Deserialize, Clone, Debug)]
pub struct TestStruct {
    pub a: u64,
    pub b: Value,
}

//rust struct for tarantool space row
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RowTypeStruct {
    pub id: u32,
    pub name: String,
    pub data: Option<TestStruct>,
}


//stored procedure function, tarantoolContext provides access for tarantool api 
fn test_iterator_impl(tarantool: &TarantoolContext) -> io::Result< Option<Vec<RowTypeStruct>>> {
    // first decode input paams, you can use Option for void params
    let (start_key, search_name_regexp): (u32, String) = tarantool.decode_input_params()?;
    //create regex for parameter
    let re = Regex::new(&search_name_regexp).unwrap();
    let mut result: Vec<RowTypeStruct> = Vec::new();

    //begi iterate over space index
    for raw_row in tarantool.index_iterator(TEST_SPACE, PRIMARY_INDEX, IteratorType::GE, &(start_key, ))? {
        //we need to check is row valid or not (take next row may return error)
        let row = raw_row?;
        //decode row field
        let name: String = row.decode_field(1)?;
        if re.is_match(&name) {
            //deserialize full row
            let row: RowTypeStruct = row.decode()?;
            result.push(row);
        }
        //yield - check any fibers to be processed
        tarantool.fiber_yield();
    };

    //return result, None for nil in LUA
    match result.len() {
        0 => Ok(None),
        _ => Ok(Some(result))
    }
}

//macro to register our function as C stored procedure 
tarantool_register_stored_procs! {
    test_iterator => test_iterator_impl
}

```

### Tarantool init script 
```lua
  box.schema.func.create('libtarantool_rust_api_example.test_bench', {language = 'C'})  
  box.schema.user.grant('guest', 'execute', 'function', 'libtarantool_rust_api_example.test_iterator')

  ...
  local capi_connection = net_box:new(3301)

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


```


