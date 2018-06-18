extern crate rusty_tarantool;

extern crate bytes;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;
extern crate futures;

extern crate rmpv;
extern crate rmp_serde;
extern crate serde;
extern crate rmp;
extern crate env_logger;

#[macro_use]
extern crate serde_derive;

use rusty_tarantool::tarantool::Client;
use tokio_core::reactor::Core;
use futures::Future;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CountryData {
    #[serde(rename = "country-code")]
    pub country_code: u32,
    pub name: String,
    pub region: String,
    #[serde(rename = "sub-region")]
    pub sub_region: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let fn_name = &args[1];
    println!("Simple client run! fn={:?}", fn_name);
    env_logger::init();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "127.0.0.1:3301".parse().unwrap();
    let client_f = Client::connect(&addr, "rust", "rust", &handle);

    let client = core.run(client_f).unwrap();
    let start = Instant::now();
    let count = 500000;


    for _ in 0..count {
        let resp = client.call_fn(fn_name, &("ru", "EUR", None::<String>))
            .and_then(move |response| {
                let res: (Vec<CountryData>, ) = response.decode()?;
                let v = COUNTER.fetch_add(1, Ordering::SeqCst);
                if v == count - 1 {
                    println!("Test  finished! count={:?} res={:?}", v, res);
                    let elapsed = start.elapsed();
                    println!("time {:?}", elapsed);
                    std::process::exit(0);
                }
                Ok(())
            })
            .map_err(|_e| ())
        ;
        handle.spawn(resp);
    };

    core.run(futures::future::empty::<(), ()>()).unwrap();
}
