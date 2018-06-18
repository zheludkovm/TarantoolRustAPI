extern crate rmp_serde;

use ::std::os::raw::{c_int, c_uchar};
use rmp_serde::{Deserializer, Serializer};

use serde::{Deserialize, Serialize};
use std::error::Error;
//use std::ffi::CStr;
use std::ffi::CString;
use std::io;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::slice;
use std::str::from_utf8_unchecked;
use tarantool::internal::*;
use tarantool::internal::StackValueType;

///Iterator tarantool type
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IteratorType {
    EQ = 0,
    REQ = 1,
    ALL = 2,
    LT = 3,
    LE = 4,
    GE = 5,
    GT = 6,
    BitsAllSet = 7,
    BitsAnySet = 8,
    BitsAllNotSet = 9,
    Ovelaps = 10,
    Neigbor = 11,
}

/// use ZERO bases indexes of args in command or ONE based (operations upsert and update)
///
/// # Examples
///
/// tarantool.update(TEST_SPACE, PRIMARY_INDEX, &key, &(("=", column, column_value), ), IndexBase::One)?;
/// column index - is 1 for first column
///
pub enum IndexBase {
    Zero = 0,
    One = 1,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FieldType {
    ANY = 0,
    UNSIGNED = 1,
    STRING = 2,
    NUMBER = 3,
    INTEGER = 4,
    BOOLEAN = 5,
    SCALAR = 6,
    ARRAY = 7,
    MAP = 8,
    MAX = 9,
}


#[repr(C)]
pub struct StoredProcCtxVal {
    _a: u8,
}

pub type StoredProcCtx = *const StoredProcCtxVal;

#[repr(C)]
pub struct StoredProcArgsVal {
    _a: u8,
}

pub type StoredProcArgs = *const StoredProcArgsVal;

#[repr(C)]
pub struct StoredProcArgsEndVal {
    _a: u8,
}

pub type StoredProcArgsEnd = *const StoredProcArgsEndVal;

const NULL: usize = 0;
const NO_KEY_SEQ: (u8,) = (0,);

const PREV_VALUE_IN_STACK: c_int = -1;

pub trait  Decodable <'ctx>{
    fn decode<'de, V>(self: &Self) -> io::Result<V> where V: Deserialize<'de>;
    fn decode_field<'de, V>(self: &Self, index: u32) -> io::Result<V> where V: Deserialize<'de>;
}

pub trait  DecodableOpt <'ctx>{
    fn decode<'de, V>(self: &Self) -> io::Result<Option<V>> where V: Deserialize<'de>;
    fn decode_field<'de, V>(self: &Self, index: u32) -> io::Result<Option<V>> where V: Deserialize<'de>;
}

#[derive(Debug)]
pub struct TarantoolTuple<'ctx> {
    pub row_data: *const u8,
    phantom: PhantomData<&'ctx TarantoolContext>,
}

impl<'ctx> TarantoolTuple<'ctx> {
    fn new(row_data: *const u8, _ctx: PhantomData<&'ctx TarantoolContext>) -> TarantoolTuple<'ctx> {
        TarantoolTuple {
            row_data,
            phantom: PhantomData,
        }
    }
}

impl<'ctx>  Decodable<'ctx> for TarantoolTuple<'ctx> {
    fn decode<'de, V>(self: &TarantoolTuple<'ctx>) -> io::Result<V>
        where V: Deserialize<'de>
    {
        unsafe {
            let size = box_tuple_bsize(self.row_data);
            let row_buf: Vec<u8> = vec![0; size];

            let _real_size = box_tuple_to_buf(self.row_data, row_buf.as_ptr(), size);
            return decode_serde(&row_buf[..]);
        }
    }

    fn decode_field<'de, V>(self: &TarantoolTuple<'ctx>, index: u32) -> io::Result<V>
        where V: Deserialize<'de>
    {
        unsafe {
            let row_buf = box_tuple_field(self.row_data, index as c_int);
            let row_buf_slice = slice::from_raw_parts(row_buf, usize::max_value());
            return decode_serde(row_buf_slice);
        }
    }
}

impl<'ctx>  DecodableOpt<'ctx> for Option<TarantoolTuple<'ctx>> {
    fn decode<'de, V>(self: &Self) ->  io::Result<Option<V>> where V: Deserialize<'de> {
        match self {
            None =>  Ok(None),
            Some(v) => v.decode()
        }
    }

    fn decode_field<'de, V>(self: &Self, index: u32) -> io::Result<Option<V>> where V: Deserialize<'de> {
        match self {
            None =>  Ok(None),
            Some(v) => v.decode_field(index)
        }
    }
}

#[derive(Debug)]
pub struct TarantoolIterator<'ctx> {
    data: *const u8,
    _params: Vec<u8>,
    //store params to keep them in memory
    ctx: PhantomData<&'ctx TarantoolContext>,
}

impl<'ctx> TarantoolIterator<'ctx> {
    fn new(data: *const u8, params: Vec<u8>, _ctx: &'ctx TarantoolContext) -> TarantoolIterator<'ctx> {
        TarantoolIterator { data, _params: params, ctx: PhantomData }
    }
}

impl<'ctx> Drop for TarantoolIterator<'ctx> {
    fn drop(&mut self) {
        unsafe {
            box_iterator_free(self.data);
        }
    }
}

impl<'ctx> Iterator for TarantoolIterator<'ctx> {
    type Item = io::Result<TarantoolTuple<'ctx>>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut ptr_buffer: *mut u8 = mem::uninitialized();
            let r = box_iterator_next(self.data, &mut ptr_buffer);
            if r == -1 {
                return Option::Some(Err(io::Error::new(io::ErrorKind::Other, "error on receive iterator next value")));
            }

            if ptr_buffer.is_null() {
                Option::None
            } else {
                Option::Some(Ok(TarantoolTuple::new(ptr_buffer, self.ctx)))
            }
        }
    }
}

#[derive(Debug)]
pub struct LuaCall<'ctx> {
    parameters_count: i32,
    lua_state: *const c_int,
    ctx: PhantomData<&'ctx TarantoolContext>,
}

impl<'ctx> LuaCall<'ctx> {
    fn new(_ctx: &'ctx TarantoolContext, fn_name: &str) -> io::Result<LuaCall<'ctx>> {
        unsafe {
            let lua_state = luaT_state();
            let fn_name_cstr = CString::new(fn_name.as_bytes())?;
            lua_getfield(lua_state, LUA_GLOBALSINDEX, fn_name_cstr.as_ptr() as *const c_uchar);
            Ok(LuaCall { lua_state, ctx: PhantomData, parameters_count: 0 })
        }
    }

    fn increment_param_count(self: &mut Self) {
        self.parameters_count = self.parameters_count + 1;
    }

    pub fn push_int(self: &mut Self, value: i64) {
        unsafe {
            self.increment_param_count();
            luaL_pushint64(self.lua_state, value);
        }
    }

    pub fn push_int_opt(self: &mut Self, value: Option<i64>) {
        match value {
            Some(value) => self.push_int(value),
            None => self.push_nil()
        }
    }

    pub fn push_uint(self: &mut Self, value: u64) {
        unsafe {
            self.increment_param_count();
            luaL_pushuint64(self.lua_state, value);
        }
    }

    pub fn push_uint_opt(self: &mut Self, value: Option<u64>) {
        match value {
            Some(value) => self.push_uint(value),
            None => self.push_nil()
        }
    }

    pub fn push_bool(self: &mut Self, value: bool) {
        unsafe {
            self.increment_param_count();
            lua_pushboolean(self.lua_state, value as c_int);
        }
    }

    pub fn push_bool_opt(self: &mut Self, value: Option<bool>) {
        match value {
            Some(value) => self.push_bool(value),
            None => self.push_nil()
        }
    }

    pub fn push_nil(self: &mut Self) {
        unsafe {
            self.increment_param_count();
            lua_pushnil(self.lua_state);
        }
    }

    pub fn push_tuple<SER>(self: &mut Self, value: &SER) -> io::Result<()> where SER: Serialize {
        unsafe {
            self.increment_param_count();
            let format = box_tuple_format_default();
            let (ptr_start, ptr_end, _buf) = serialize_to_ptr(value)?;
            let tuple = box_tuple_new(format, ptr_start, ptr_end);
            luaT_pushtuple(self.lua_state, tuple);
            Ok(())
        }
    }

    pub fn push_tuple_opt<SER>(self: &mut Self, value: &Option<SER>) -> io::Result<()> where SER: Serialize {
        match value {
            Some(value) => self.push_tuple(value),
            None => {
                self.push_nil();
                Ok(())
            }
        }
    }

    pub fn push_str<S>(self: &mut Self, value: S)
        where S: AsRef<[u8]>
    {
        unsafe {
            self.parameters_count = self.parameters_count + 1;
            let value_b = value.as_ref();
            lua_pushlstring(self.lua_state, value_b.as_ptr(), value_b.len());
        }
    }

    pub fn push_str_opt<S>(self: &mut Self, value: &Option<S>)
        where S: AsRef<[u8]>
    {
        match value {
            Some(value) => self.push_str(value),
            None => self.push_nil()
        }
    }


    pub fn call(self: &Self) -> io::Result<()> {
        unsafe {
            if luaT_call(self.lua_state, self.parameters_count, -1) != 0 {
                return make_error(format!("error on call stored proc!  ", ));
            }
        }
        Ok(())
    }

    fn get_value_from_stack<T>(self: &Self, expecting_type: StackValueType, f: fn(lua_state: *const c_int) -> io::Result<T>) -> io::Result<Option<T>> {
        unsafe {
            let stack_value_type = lua_type(self.lua_state, PREV_VALUE_IN_STACK);
            let expecting_type_i32: i32 = expecting_type as i32;
            match stack_value_type {
                v if v == expecting_type_i32 => {
                    let res = f(self.lua_state);
                    lua_pop_and_return(self.lua_state, Some(res?))
                }
                v if v == LUA_TNIL => lua_pop_and_return(self.lua_state, None),
                _ => make_error(format!("Incorrect type of result!  expecting {:?} receive {:?}", expecting_type.to_string(), StackValueType::raw_to_string(stack_value_type as u32))),
            }
        }
    }


    pub fn pop_number(self: &Self) -> io::Result<Option<f64>> {
        self.get_value_from_stack(StackValueType::LUA_TNUMBER, lua_tonumber_wrapper)
    }

    pub fn pop_integer(self: &Self) -> io::Result<Option<i64>> {
        self.get_value_from_stack(StackValueType::LUA_TNUMBER, lua_tointeger_wrapper)
    }

    pub fn pop_boolean(self: &Self) -> io::Result<Option<bool>> {
        self.get_value_from_stack(StackValueType::LUA_TBOOLEAN, lua_toboolean_wrapper)
    }

    pub fn pop_str(self: &Self) -> io::Result<Option<String>> {
        self.get_value_from_stack(StackValueType::LUA_TSTRING, lua_tolstring_wrapper)
    }

    fn lua_istuple_wrapper<'a>(lua_state: *const c_int) -> io::Result<TarantoolTuple<'a>> {
        unsafe {
            let res_tuple = luaT_istuple(lua_state, PREV_VALUE_IN_STACK);
            if res_tuple as usize == NULL {
                return make_error(format!("No tuple as ret param!", ));
            }
            Ok(TarantoolTuple::new(res_tuple, PhantomData))
        }
    }

    pub fn pop_tuple(self: &Self) -> io::Result<Option<TarantoolTuple>> {
        self.get_value_from_stack(StackValueType::LUA_TUPLE, Self::lua_istuple_wrapper)
    }
}

#[derive(Debug)]
pub struct TarantoolContext {
    context: StoredProcCtx,
    args: StoredProcArgs,
    args_end: StoredProcArgsEnd,
}

impl TarantoolContext {
    pub fn new(context: StoredProcCtx,
               args: StoredProcArgs,
               args_end: StoredProcArgsEnd) -> TarantoolContext {
        TarantoolContext { context, args, args_end }
    }


    pub fn decode_input_params<'de, T>(self: &TarantoolContext) -> io::Result<T>
        where T: Deserialize<'de>
    {
        unsafe {
            let size = self.args_end as usize - self.args as usize;
            let slice = slice::from_raw_parts(self.args as *const u8, size);
            decode_serde(slice).map_err(|_e| {
                io::Error::new(io::ErrorKind::Other, "Can't decode input parameters of Rust stored procedure ! Incorrect format of input message!")
            })
        }
    }

    pub fn index_iterator_all<'a,  S, S1>(self: &'a Self, space_name: S, index_name: S1) -> io::Result<TarantoolIterator>
        where S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        self.index_iterator(space_name,index_name, IteratorType::ALL , &NO_KEY_SEQ)
    }

    pub fn index_iterator<'a, SER, S, S1>(self: &'a Self, space_name: S, index_name: S1, iterator_type: IteratorType, key: &SER) -> io::Result<TarantoolIterator>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        unsafe {
            let (space_id, index_id) = get_space_and_index_id(&space_name, &index_name)?;

            let (ptr_start, ptr_end, params) = serialize_to_ptr(key)?;
            let iter = box_index_iterator(space_id, index_id, iterator_type as u8, ptr_start, ptr_end);
            if iter as usize == NULL {
                return make_error(format!("space name={} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name.as_ref())));
            }
            Ok(TarantoolIterator::new(iter, params, self))
        }
    }

    pub fn insert<'a, SER, S>(self: &'a Self, space_name: S, value: &SER) -> io::Result<()>
        where SER: Serialize,
              S: AsRef<[u8]>
    {
        unsafe {
            let (ptr_start, ptr_end, _params) = serialize_to_ptr(value)?;
            let space_id = get_space_id(&space_name)?;
            let res = box_insert(space_id, ptr_start, ptr_end, ptr::null_mut());
            if res == -1 {
                return make_error(format!("error on insert! space name={:?} ", from_utf8_unchecked(space_name.as_ref())));
            }
            Ok(())
        }
    }

    pub fn replace<'a, SER, S>(self: &'a Self, space_name: S, value: &SER) -> io::Result<()>
        where SER: Serialize,
              S: AsRef<[u8]>
    {
        unsafe {
            let (ptr_start, ptr_end, _params) = serialize_to_ptr(value)?;
            let space_id = get_space_id(&space_name)?;
            let res = box_replace(space_id, ptr_start, ptr_end, ptr::null_mut());
            if res == -1 {
                return make_error(format!("error on replace! space name={:?} ", from_utf8_unchecked(space_name.as_ref())));
            }
            Ok(())
        }
    }

    pub fn delete<'a, SER, S, S1>(self: &'a Self, space_name: S, index_name: S1, key: &SER) -> io::Result<()>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        unsafe {
            let (space_id, index_id) = get_space_and_index_id(&space_name, &index_name)?;

            let (ptr_start, ptr_end, _params) = serialize_to_ptr(key)?;

            let res = box_delete(space_id, index_id, ptr_start, ptr_end, ptr::null_mut());
            if res == -1 {
                return make_error(format!("error on delete! space name={} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name.as_ref())));
            }
            Ok(())
        }
    }

    pub fn update<'a, SER, SER1, S, S1>(self: &'a Self, space_name: S, index_name: S1, key: &SER, ops: &SER1, index_base: IndexBase) -> io::Result<()>
        where SER: Serialize,
              SER1: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        unsafe {
            let (space_id, index_id) = get_space_and_index_id(&space_name, &index_name)?;

            let (key_ptr_start, key_ptr_end, _key_params) = serialize_to_ptr(key)?;
            let (ops_ptr_start, ops_ptr_end, _ops_params) = serialize_to_ptr(ops)?;

            let res = box_update(space_id, index_id, key_ptr_start, key_ptr_end, ops_ptr_start, ops_ptr_end, index_base as i32, ptr::null_mut());
            if res == -1 {
                return make_error(format!("error on update! space name={} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name.as_ref())));
            }
            Ok(())
        }
    }

    pub fn upsert<'a, SER, SER1, S, S1>(self: &'a Self, space_name: S, index_name: S1, tuple: &SER, ops: &SER1, index_base: IndexBase) -> io::Result<()>
        where SER: Serialize,
              SER1: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        unsafe {
            let (space_id, index_id) = get_space_and_index_id(&space_name, &index_name)?;

            let (tuple_ptr_start, tuple_ptr_end, _tuple_params) = serialize_to_ptr(tuple)?;
            let (ops_ptr_start, ops_ptr_end, _ops_params) = serialize_to_ptr(ops)?;

            let res = box_upsert(space_id, index_id, tuple_ptr_start, tuple_ptr_end, ops_ptr_start, ops_ptr_end, index_base as i32, ptr::null_mut());
            if res == -1 {
                return make_error(format!("error on upsert! space name={} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name.as_ref())));
            }
            Ok(())
        }
    }

    pub fn truncate_space<'a, S>(self: &'a Self, space_name: S) -> io::Result<()>
        where S: AsRef<[u8]>
    {
        unsafe {
            let space_id = get_space_id(&space_name)?;

            let res = box_truncate(space_id);
            if res == -1 {
                return make_error(format!("error on truncate space! space name={}  ", from_utf8_unchecked(space_name.as_ref())));
            }
            Ok(())
        }
    }

    pub fn sequence_next<'a, S>(self: &'a Self, sequence_name: S) -> io::Result<i64>
        where S: AsRef<[u8]>
    {
        unsafe {
            let next_value: i64 = 0;
            let res = box_sequence_next(2, &next_value);
            if res == -1 {
                return make_error(format!("error on get sequence value! sequnce name={}  ", from_utf8_unchecked(sequence_name.as_ref())));
            }
            Ok(next_value)
        }
    }


    fn index_get_any<'a, SER, S, S1>(self: &'a Self,
                                     space_name: S,
                                     index_name: S1,
                                     key: &SER,
                                     f: unsafe extern "C" fn(u32, u32, *const c_uchar, *const c_uchar, *mut (*mut c_uchar)) -> c_int) -> io::Result<Option<TarantoolTuple>>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>,

    {
        unsafe {
            let (space_id, index_id) = get_space_and_index_id(&space_name, &index_name)?;

            let (key_start, key_end, _params) = serialize_to_ptr(key)?;
            let mut res_tuple: *mut u8 = mem::uninitialized();

            let res = f(space_id, index_id, key_start, key_end, &mut res_tuple);
            if res == -1 {
                return make_error(format!("error on get data ! space name={} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name.as_ref())));
            }
            if res_tuple as usize==NULL {
               return Ok(None)
            }

            Ok(Some(TarantoolTuple::new(res_tuple, PhantomData)))
        }
    }

    pub fn index_get<'a, SER, S, S1>(self: &'a Self, space_name: S, index_name: S1, key: &SER) -> io::Result<Option<TarantoolTuple>>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        self.index_get_any(space_name, index_name, key, box_index_get)
    }

    pub fn index_min<'a, SER, S, S1>(self: &'a Self, space_name: S, index_name: S1, key: &SER) -> io::Result<Option<TarantoolTuple>>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        self.index_get_any(space_name, index_name, key, box_index_min)
    }

    pub fn index_max<'a, SER, S, S1>(self: &'a Self, space_name: S, index_name: S1, key: &SER) ->  io::Result<Option<TarantoolTuple>>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        self.index_get_any(space_name, index_name, key, box_index_max)
    }

    pub fn index_count<'a, SER, S, S1>(self: &'a Self, space_name: S, index_name: S1, iterator_type: IteratorType, key: &SER) -> io::Result<isize>
        where SER: Serialize,
              S: AsRef<[u8]>,
              S1: AsRef<[u8]>
    {
        unsafe {
            let (space_id, index_id) = get_space_and_index_id(&space_name, &index_name)?;

            let (key_start, key_end, _params) = serialize_to_ptr(key)?;

            let res = box_index_count(space_id, index_id, iterator_type as u8, key_start, key_end);
            if res == -1 {
                return make_error(format!("error on index count! space name={} index name={} ", from_utf8_unchecked(space_name.as_ref()), from_utf8_unchecked(index_name.as_ref())));
            }
            Ok(res)
        }
    }

    pub fn txn_id(self: &Self) -> i64 {
        unsafe {
            box_txn_id()
        }
    }

    pub fn txn_begin(self: &Self) -> io::Result<()> {
        unsafe {
            match box_txn_begin() {
                -1 => make_error(format!("error on begin tranaction ")),
                _ => Ok(())
            }
        }
    }

    pub fn txn_commit(self: &Self) -> io::Result<()> {
        unsafe {
            match box_txn_commit() {
                -1 => make_error(format!("error on begin tranaction ")),
                _ => Ok(())
            }
        }
    }

    pub fn txn_rollback(self: &Self) -> io::Result<()> {
        unsafe {
            match box_txn_rollback() {
                -1 => make_error(format!("error on begin tranaction ")),
                _ => Ok(())
            }
        }
    }

    pub fn fiber_yield(self: &Self) {
        unsafe {
            fiber_sleep(0 as f64);
        }
    }

    pub fn fiber_sleep(self: &Self, time: f64) {
        unsafe {
            fiber_sleep(time);
        }
    }

    pub fn init_call<'a>(self: &'a Self, fn_name: &str) -> io::Result<LuaCall<'a>> {
        LuaCall::new(self, fn_name)
    }

    pub fn return_tuple<'a, SER>(self: &'a Self, result: io::Result<SER>, format: Option<&Vec<FieldType>>) -> c_int
        where SER: Serialize
    {
        match result {
            Ok(ref value) => {
                unsafe {
                    let tuple_format = match format {
                        None => box_tuple_format_default(),
                        Some(fields) => {
                            let fields_n: Vec<u32> = (0..fields.len()).map(|v| v as u32).collect();
                            let key_def = box_key_def_new(fields_n.as_ptr(), fields.as_ptr() as *const u32, fields.len() as u32);
                            box_tuple_format_new(&key_def, 1 as u16)
                        }
                    };

                    let (ptr_start, ptr_end, _buf) = serialize_to_ptr(value).unwrap();
                    let tuple = box_tuple_new(tuple_format, ptr_start, ptr_end);
                    if tuple as usize == NULL {
                        make_error::<String>(format!("error on create tuple!")).unwrap();
                    }
                    let res = box_return_tuple(self.context as *const u8, tuple);
                    return res;
                }
            }
            Err(ref error) => {
                let _set_last_error_res = set_last_error_wrapper(error.description());
                print!("Error {}", error);
                return -1;
            }
        }
    }
}

pub fn exec_stored_procedure<F, SER>(context: StoredProcCtx, args: StoredProcArgs, args_end: StoredProcArgsEnd, f: F) -> c_int
    where F: FnOnce(&TarantoolContext) -> io::Result<SER>,
          SER: Serialize
{
    let tarantool = TarantoolContext::new(context, args, args_end);
    return tarantool.return_tuple(f(&tarantool), None);
}

pub fn exec_stored_procedure_with_format<F, SER>(
    context: StoredProcCtx,
    args: StoredProcArgs,
    args_end: StoredProcArgsEnd,
    f: F,
    format: &Vec<FieldType>) -> c_int
    where F: FnOnce(&TarantoolContext) -> io::Result<SER>,
          SER: Serialize
{
    let tarantool = TarantoolContext::new(context, args, args_end);
    return tarantool.return_tuple(f(&tarantool), Some(format));
}


fn decode_serde<'de, T, R>(r: R) -> io::Result<T>
    where T: Deserialize<'de>, R: io::Read
{
    Deserialize::deserialize(&mut Deserializer::new(r)).map_err(map_err_to_io)
}

fn serialize_to_ptr<S: Serialize>(v: &S) -> io::Result<(*const u8, *const u8, Vec<u8>)> {
    unsafe {
         let mut buf = Vec::new();
        serialize_to_buf_mut(&mut buf, v)?;
        let ptr_start = buf.as_ptr();
        let ptr_end = ptr_start.offset(buf.len() as isize);
        Ok((ptr_start, ptr_end, buf))
    }
}

pub fn serialize_to_buf_mut<W: io::Write, S: Serialize>(wr: &mut W, v: &S) -> io::Result<()> {
//    v.serialize(&mut Serializer::new(wr).with_struct_map().with_struct_map()).map_err(map_err_to_io)
    v.serialize(&mut Serializer::new(wr)).map_err(map_err_to_io)
}


