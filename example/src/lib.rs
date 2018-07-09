extern crate regex;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate tarantool_rust_api;

use regex::Regex;
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_json::Value;
use std::ffi::CStr;
use std::io;
use std::os::raw::{c_char, c_int};
use std::vec::Vec;
use tarantool_rust_api::tarantool::api::*;

static TEST_SPACE: &str = "test_space";
static PRIMARY_INDEX: &str = "primary";
static SECONDARY_INDEX: &str = "secondary";

#[no_mangle]
pub fn test_ffi(v: c_int, ptr: *const c_char) -> c_int {
    let c_str: &CStr = unsafe { CStr::from_ptr(ptr) };
    println!("test {} len={:?}", v, c_str.to_str().unwrap());
    v + 1
}

#[derive(Deserialize, Clone, Debug)]
pub struct TestStruct {
    pub a: u64,
    pub b: Value,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TestStructSeq {
    pub a: u64,
    pub b: Value,
    pub c: TestStruct,
}

impl Serialize for TestStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("a", &self.a)?;
        map.serialize_entry("b", &self.b)?;
        map.end()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RowTypeStruct {
    pub id: u32,
    pub name: String,
    pub data: Option<TestStruct>,
}

pub fn test_insert_impl(tarantool: &TarantoolContext) -> io::Result<bool> {
    let val: RowTypeStruct = tarantool.decode_input_params()?;
    tarantool.txn_begin()?;
    tarantool.insert(TEST_SPACE, &val)?;
    tarantool.txn_commit()?;
    Ok(true)
}

fn test_replace_impl(tarantool: &TarantoolContext) -> io::Result<bool> {
    let val: RowTypeStruct = tarantool.decode_input_params()?;
    tarantool.replace(TEST_SPACE, &val)?;
    Ok(true)
}

fn test_index_get_impl(tarantool: &TarantoolContext) -> io::Result<Option<RowTypeStruct>> {
    let key: (u32, ) = tarantool.decode_input_params()?;
    match tarantool.index_get(TEST_SPACE, PRIMARY_INDEX, &key)? {
        Some(tuple) => Ok(tuple.decode()?),
        None => Ok(None)
    }
}

fn test_delete_impl(tarantool: &TarantoolContext) -> io::Result<bool> {
    let key: (u32, ) = tarantool.decode_input_params()?;
    tarantool.delete(TEST_SPACE, PRIMARY_INDEX, &key)?;
    Ok(true)
}

fn test_update_impl(tarantool: &TarantoolContext) -> io::Result<bool> {
    let (key, column, column_value): ((u32, ), u32, TestStruct) = tarantool.decode_input_params()?;
    tarantool.update(TEST_SPACE, PRIMARY_INDEX, &key, &(("=", column, column_value), ), IndexBase::One)?;
    Ok(true)
}

fn test_upsert_impl(tarantool: &TarantoolContext) -> io::Result<bool> {
    let (row, column, column_value): (RowTypeStruct, u32, TestStruct) = tarantool.decode_input_params()?;
    tarantool.upsert(TEST_SPACE, PRIMARY_INDEX, &row, &(("=", column, column_value), ), IndexBase::One)?;
    Ok(true)
}

fn test_iterator_impl(tarantool: &TarantoolContext) -> io::Result<Option<Vec<RowTypeStruct>>> {
    let (start_key, search_name_regexp): (u32, String) = tarantool.decode_input_params()?;
    let re = Regex::new(&search_name_regexp).unwrap();
    let mut result: Vec<RowTypeStruct> = Vec::new();

    for raw_row in tarantool.index_iterator(TEST_SPACE, PRIMARY_INDEX, IteratorType::GE, &(start_key, ))? {
        let row = raw_row?;
        let name: String = row.decode_field(1)?;
        if re.is_match(&name) {
            let row: RowTypeStruct = row.decode()?;
            result.push(row);
        }
        tarantool.fiber_yield();
    };

    match result.len() {
        0 => Ok(None),
        _ => Ok(Some(result))
    }
}

fn test_min_max_count_impl(tarantool: &TarantoolContext) -> io::Result<(Option<RowTypeStruct>, Option<RowTypeStruct>, isize)> {
    let (start_name, ): (String, ) = tarantool.decode_input_params()?;
    let start_key = &(start_name, );

    let min = tarantool.index_min(TEST_SPACE, SECONDARY_INDEX, &start_key)?.decode()?;
    let max = tarantool.index_max(TEST_SPACE, SECONDARY_INDEX, &start_key)?.decode()?;
    let count = tarantool.index_count(TEST_SPACE, SECONDARY_INDEX, IteratorType::GE, &start_key)?;

    Ok((min, max, count))
}

fn test_truncate_impl(tarantool: &TarantoolContext) -> io::Result<bool> {
    tarantool.truncate_space(TEST_SPACE)?;
    Ok(true)
}

fn test_lua_call_impl(tarantool: &TarantoolContext) -> io::Result<(Option<TestStructSeq>, Option<String>, Option<i64>, Option<bool>, Option<bool>)> {
    let (p_nil, p_num, p_str, p_tuple): (Option<String>, i64, String, TestStructSeq) = tarantool.decode_input_params()?;
    let mut call = tarantool.init_call("test_fn")?;
    call.push_str_opt(&p_nil);
    call.push_int(p_num);
    call.push_str(p_str);
    call.push_tuple_opt(&Some(p_tuple))?;
    call.call()?;
    let res_tuple: Option<TestStructSeq> = call.pop_tuple()?.decode()?;
    let res_str = call.pop_str()?;
    let res_num = call.pop_integer()?;
    let res_true = call.pop_boolean()?;
    let res_zero = call.pop_boolean()?;

    Ok((res_tuple, res_str, res_num, res_true, res_zero))
}


#[derive(Deserialize, Clone, Debug)]
pub struct CountryData {
    pub country_code: u32,
    pub name: String,
    pub region: String,
    pub sub_region: String,
}

impl Serialize for CountryData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("country-code", &self.country_code)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("region", &self.region)?;
        map.serialize_entry("sub-region", &self.sub_region)?;
        map.end()
    }
}

static COUNTRY_SPACE: &str = "countries";
static COUNTRY_INDEX: &str = "primary";

fn check_attr(search_str: &Option<String>, row: &TarantoolTuple, index: u32) -> io::Result<bool> {
    match search_str {
        None => Ok(true),
        Some(search_str_v) => {
            let column_value: Option<String> = row.decode_field(index)?;
            match column_value {
                None => Ok(false),
                Some(column_value_v) => {
                    Ok(column_value_v.contains(search_str_v))
                }
            }
        }
    }
}

fn test_bench_impl(tarantool: &TarantoolContext) -> io::Result<Vec<CountryData>> {
    let (p_name, p_region, p_sub_region): (Option<String>, Option<String>, Option<String>) = tarantool.decode_input_params()?;
    let p_name_l = p_name.map(|v|v.to_lowercase());
    let p_region_l = p_region.map(|v|v.to_lowercase());
    let p_sub_region_l = p_sub_region.map(|v|v.to_lowercase());

    let mut result: Vec<CountryData> = Vec::new();
    for raw_row in tarantool.index_iterator_all(COUNTRY_SPACE, COUNTRY_INDEX)? {
        let row = raw_row?;
        if check_attr(&p_name_l, &row, 1)? &&
            check_attr(&p_region_l, &row, 2)? &&
            check_attr(&p_sub_region_l, &row, 3)? {
            let row: CountryData = row.decode()?;
            result.push(row);
        }
//        tarantool.fiber_yield();
    };

    Ok(result)
}

fn test_get_space_id_impl(tarantool: &TarantoolContext) -> io::Result<u32> {
    let (space_name,) : (String,) = tarantool.decode_input_params()?;
    return tarantool.get_space_id(space_name);
}

tarantool_register_stored_procs! {
    test_insert => test_insert_impl,
    test_index_get => test_index_get_impl,
    test_replace => test_replace_impl,
    test_delete => test_delete_impl,
    test_update => test_update_impl,
    test_upsert => test_upsert_impl,
    test_iterator => test_iterator_impl,
    test_min_max_count => test_min_max_count_impl,
    test_truncate => test_truncate_impl,
    test_lua_call => test_lua_call_impl,
    test_bench => test_bench_impl,
    test_get_space_id => test_get_space_id_impl
}


//#[no_mangle]
//pub fn test_index_get(context: StoredProcCtx, args: StoredProcArgs, args_end: StoredProcArgsEnd) -> c_int {
//    exec_stored_procedure(context, args, args_end, test_index_get_impl)
//}


