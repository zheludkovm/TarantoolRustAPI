#[macro_export]
macro_rules! tarantool_stored_proc {
    ( pub fn $fn_name:ident $params:tt $res_expr:tt $res:ty $body:block) => {
        tarantool_stored_proc!(fn $fn_name $params $res_expr $res $body );
    };

    ( fn $fn_name:ident $params:tt $res_expr:tt $res:ty $body:block) => {
        #[no_mangle]
        pub fn $fn_name(context: tarantool_rust_api::tarantool::api::StoredProcCtx,
                        args: tarantool_rust_api::tarantool::api::StoredProcArgs,
                        args_end: tarantool_rust_api::tarantool::api::StoredProcArgsEnd ) -> std::os::raw::c_int  {
            fn tmp $params $res_expr $res $body ;
            let tarantool = TarantoolContext::new(context, args, args_end);
            return tarantool.return_tuple(tmp(&tarantool), None);
        }
    }
}

#[macro_export]
macro_rules! tarantool_register_stored_procs {
    ($( $export_fn:ident => $impl_fn:ident  ),* ) => {
        $(
            #[no_mangle]
             pub fn $export_fn(context: tarantool_rust_api::tarantool::api::StoredProcCtx,
                    args: tarantool_rust_api::tarantool::api::StoredProcArgs,
                    args_end: tarantool_rust_api::tarantool::api::StoredProcArgsEnd ) -> std::os::raw::c_int  {
                let tarantool = TarantoolContext::new(context, args, args_end);
                return tarantool.return_tuple($impl_fn(&tarantool), None);
            }
        )*

        #[no_mangle]
        pub fn init_dictionaries_ffi(){
            init_dictionaries().unwrap();
        }
    };
}

//tarantool_register_stored_procs! {
//    test_index_get => test_index_get_impl,
//    test_replace => test_replace_impl
//}


//tarantool_stored_proc! {
//     pub fn test_insert(tarantool: &TarantoolContext) -> io::Result<bool> {
//        let val: RowTypeStruct = tarantool.decode_input_params()?;
//        println!("val={:?} ", val);
//        tarantool.insert("test_space", &val)?;
//        Ok(true)
//     }
//}
