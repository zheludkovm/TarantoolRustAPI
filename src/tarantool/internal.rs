use ::std::os::raw::{c_char, c_int, c_uchar};
use std::error;
use std::ffi::{CStr, CString};
use std::io;
use std::slice;
use std::str::from_utf8_unchecked;

#[allow(unused_variables)]

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[allow(unused_variables)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub enum StackValueType {
    LUA_TNIL = 0,
    LUA_TBOOLEAN = 1,
    LUA_TLIGHTUSERDATA = 2,
    LUA_TNUMBER = 3,
    LUA_TSTRING = 4,
    LUA_TTABLE = 5,
    LUA_TFUNCTION = 6,
    LUA_TUSERDATA = 7,
    LUA_TTHREAD = 8,
    LUA_TUPLE = 10,
}

impl StackValueType {
    pub fn raw_to_string(value: u32) -> &'static str {
        match value{
            0 => "NIL",
            1 => "BOOLEAN",
            2 => "LIGHTUSERDATA",
            3 => "NUMBER",
            4 => "STRING",
            5 => "TABLE",
            6 => "FUNCTION",
            7 => "USERDATA",
            8 => "THREAD",
            10 => "TUPLE",
            _ => "unknown type"
        }
    }

    pub fn to_string(self: &Self) -> &'static str {
        Self::raw_to_string(*self as u32)
    }
}

pub const LUA_TNIL: i32 = StackValueType::LUA_TNIL as i32;

pub const BOX_ID_NIL: u32 = 2147483647;
pub const LUA_GLOBALSINDEX: c_int = -10002;
//pub const BOX_SEQUENCE_ID:u32 = 284;

#[allow(dead_code)]
extern "C" {
    pub fn box_return_tuple(ctx: *const c_uchar, tuple: *const c_uchar) -> c_int;

    pub fn box_index_iterator(space_id: u32, index_id: u32, p_type: c_uchar, key: *const c_uchar, key_end: *const c_uchar) -> *const c_uchar;
    pub fn box_iterator_next(box_iterator_t: *const c_uchar, box_tuple_t: *mut (*mut c_uchar)) -> c_int;
    pub fn box_iterator_free(box_iterator_t: *const c_uchar);

    pub fn box_insert(space_id: u32, tuple: *const c_uchar, tuple_end: *const c_uchar, result: *mut (*mut c_uchar)) -> c_int;
    pub fn box_replace(space_id: u32, tuple: *const c_uchar, tuple_end: *const c_uchar, result: *mut (*mut c_uchar)) -> c_int;
    pub fn box_delete(space_id: u32, index_id: u32, key: *const c_uchar, key_end: *const c_uchar, result: *mut (*mut c_uchar)) -> c_int;
    pub fn box_update(space_id: u32, index_id: u32, key: *const c_uchar, key_end: *const c_uchar, ops: *const c_uchar, ops_end: *const c_uchar, index_base: i32, result: *mut (*mut c_uchar)) -> c_int;
    pub fn box_upsert(space_id: u32, index_id: u32, key: *const c_uchar, key_end: *const c_uchar, ops: *const c_uchar, ops_end: *const c_uchar, index_base: i32, result: *mut (*mut c_uchar)) -> c_int;
    pub fn box_truncate(space_id: u32) -> c_int;
    pub fn box_sequence_next(seq_id: u32, result: *const i64) -> c_int;

    pub fn box_index_get(space_id: u32, index_id: u32, key: *const c_uchar, key_end: *const c_uchar, box_tuple_t: *mut (*mut c_uchar)) -> c_int;
    pub fn box_index_min(space_id: u32, index_id: u32, key: *const c_uchar, key_end: *const c_uchar, box_tuple_t: *mut (*mut c_uchar)) -> c_int;
    pub fn box_index_max(space_id: u32, index_id: u32, key: *const c_uchar, key_end: *const c_uchar, box_tuple_t: *mut (*mut c_uchar)) -> c_int;
    pub fn box_index_count(space_id: u32, index_id: u32, p_type: c_uchar, key: *const c_uchar, key_end: *const c_uchar) -> isize;


    pub fn box_key_def_new(fields: *const u32, types: *const u32, part_count: u32) -> *const c_uchar;
    pub fn box_tuple_format_new(keys: *const ( *const c_uchar), key_count: u16) -> *const c_uchar;
    pub fn box_tuple_format_default() -> *const c_uchar;
    pub fn box_tuple_new(format: *const c_uchar, data: *const c_uchar, end: *const c_uchar) -> *const c_uchar;
    pub fn box_tuple_field(box_tuple_t: *const c_uchar, fieldno: c_int) -> *const c_uchar;
    pub fn box_tuple_bsize(box_tuple_t: *const c_uchar) -> usize;
    pub fn box_tuple_to_buf(box_tuple_t: *const c_uchar, buf: *const c_uchar, size: usize) -> usize;

    pub fn box_space_id_by_name(name: *const c_uchar, len: u32) -> u32;
    pub fn box_index_id_by_name(space_id: u32, name: *const c_uchar, len: u32) -> u32;

    pub fn box_txn_begin() -> c_int;
    pub fn box_txn_commit() -> c_int;
    pub fn box_txn_rollback() -> c_int;
    pub fn box_txn_id() -> i64;

    //    pub fn box_error_code(box_error_t: *const c_uchar) -> u32;
    pub fn box_error_message(box_error_t: *const c_uchar) -> *const c_char;
    pub fn box_error_last() -> *const c_uchar;
    pub fn box_error_set(file: *const c_char, line: u32, code: u32, message: *const c_char) -> *const u32;

    //lua integration
    pub fn luaT_state() -> *const c_int;
    pub fn luaL_pushint64(lua_state: *const c_int, val: i64);
    pub fn luaL_pushuint64(lua_state: *const c_int, val: u64);
    pub fn luaT_pushtuple(lua_state: *const c_int, tuple: *const c_uchar);
    pub fn lua_pushnil(lua_state: *const c_int);
    pub fn lua_pushlstring(lua_state: *const c_int, s: *const c_uchar, l: usize);
    pub fn lua_pushboolean(lua_state: *const c_int, val: c_int);

    //set function name
    pub fn lua_getfield(lua_state: *const c_int, idx: c_int, k: *const c_uchar);

    //call function
    pub fn luaT_call(lua_state: *const c_int, nargs: c_int, nreturns: c_int) -> c_int;
    //get functions
    pub fn lua_type(lua_state: *const c_int, idx: c_int) -> c_int;
    pub fn lua_settop(lua_state: *const c_int, idx: c_int);
    pub fn lua_gettop(lua_state: *const c_int, idx: c_int) -> c_int;
    pub fn lua_tonumber(lua_state: *const c_int, idx: c_int) -> f64;
    pub fn lua_tointeger(lua_state: *const c_int, idx: c_int) -> i64;
    pub fn lua_toboolean(lua_state: *const c_int, idx: c_int) -> c_int;
    pub fn lua_isnumber(lua_state: *const c_int, idx: c_int) -> c_int;
    pub fn lua_tolstring(lua_state: *const c_int, idx: c_int, len: *mut usize) -> *const c_uchar;
    pub fn luaT_istuple(lua_state: *const c_int, idx: c_int) -> *const c_uchar;

    pub fn fiber_yield();
    pub fn fiber_sleep(time: f64);
}

#[allow(dead_code)]
pub fn lua_pop(l: *const c_int, n: c_int) {
    unsafe {
        lua_settop(l, -(n) - 1)
    }
}

pub fn lua_pop_and_return<T>(l: *const c_int, res: T) -> io::Result<T> {
    unsafe {
        lua_settop(l, -(1) - 1);
        Ok(res)
    }
}

pub fn lua_tonumber_wrapper(lua_state: *const c_int) -> io::Result<f64> {
    unsafe {
        Ok(lua_tonumber(lua_state, -1))
    }
}

pub fn lua_tointeger_wrapper(lua_state: *const c_int) -> io::Result<i64> {
    unsafe {
        Ok(lua_tointeger(lua_state, -1))
    }
}

pub fn lua_toboolean_wrapper(lua_state: *const c_int) -> io::Result<bool> {
    unsafe {
        Ok(lua_toboolean(lua_state, -1) == 1)
    }
}

pub fn lua_tolstring_wrapper(lua_state: *const c_int) -> io::Result<String> {
    unsafe {
        let mut str_len: usize = 0;
        let str_raw_pointer = lua_tolstring(lua_state, -1, &mut str_len as *mut usize);
        let slice = slice::from_raw_parts(str_raw_pointer, str_len);
        let res_str = CString::new(slice)?;
        let res = res_str.to_str().map_err(map_err_to_io)?.to_owned();
        Ok(res)
    }
}

#[allow(dead_code)]
pub fn get_space_id<S>(space_name: S) -> io::Result<u32>
    where S: AsRef<[u8]>
{
    unsafe {
        let space_name_b = space_name.as_ref();
        let space_id = box_space_id_by_name(space_name_b.as_ptr(), space_name_b.len() as u32);
        if space_id == BOX_ID_NIL {
            return make_error(format!("unknown space name! space name={}", from_utf8_unchecked(space_name_b)));
        } else {
            return Ok(space_id);
        }
    }
}

#[allow(dead_code)]
pub fn get_index_id<S, S1>(space_name: S, space_id: u32, index_name: S1) -> io::Result<u32>
    where S: AsRef<[u8]>,
          S1: AsRef<[u8]>
{
    unsafe {
        let index_name_b = index_name.as_ref();
        let index_id = box_index_id_by_name(space_id, index_name_b.as_ptr(), index_name_b.len() as u32);
        if index_id == BOX_ID_NIL {
            return make_error(format!("unknown index name! space name={:?} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name_b)));
        } else {
            return Ok(index_id);
        }
    }
}

#[allow(dead_code)]
pub fn get_space_and_index_id<S,S1>(space_name: S,index_name: S1) -> io::Result<(u32,u32)>
    where S: AsRef<[u8]>,
          S1: AsRef<[u8]>
{
    let space_id = get_space_id(&space_name)?;
    let index_id = get_index_id(&space_name, space_id, &index_name)?;
    Ok((space_id, index_id))
}

pub fn set_last_error_wrapper(message: &str) -> io::Result<()> {
    unsafe {
        box_error_set(CString::new("rust")?.as_ptr(), 1, 1, CString::new(message.as_bytes())?.as_ptr());
    }
    Ok(())
}

pub fn make_error<T>(additional_message: String) -> io::Result<T> {
    unsafe {
        let box_error = box_error_last();
        if box_error as usize == 0 {
            Err(io::Error::new(io::ErrorKind::Other, additional_message))
        } else {
            let message = box_error_message(box_error);
            let error_message = CStr::from_ptr(message).to_str().map_err(map_err_to_io)?;
            Err(io::Error::new(io::ErrorKind::Other, format!("{}, additional info : {}", error_message, additional_message)))
        }
    }
}

pub fn map_err_to_io<E>(e: E) -> io::Error
    where E: Into<Box<error::Error + Send + Sync>>
{
//    error!("Error! {:?}", e.into());
    println!("Error {:?}", e.into());
    io::Error::new(io::ErrorKind::Other, "")
}


