use rusty_tarantool::tarantool::ClientConfig;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use serde::{Deserialize, Serialize};

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

#[tokio::main(core_threads = 4)]
pub async fn main() -> std::io::Result<()>{
    // #[tokio::main]

    // let threaded_rt = tokio::runtime::Builder::new()
    //     .threaded_scheduler()
    //     .build()?;

    let args: Vec<String> = std::env::args().collect();
    let fn_name = String::from(&args[1]);
    println!("Simple client run! fn={:?}", fn_name);
    env_logger::init();

    let client = ClientConfig::new("127.0.0.1:3301", "rust", "rust")
        .set_timeout_time_ms(2000)
        .set_reconnect_time_ms(2000)
        .build();

    let start = Instant::now();
    let count = 20000;
    let mut handles = vec![];

    for _ in 0..count {
        let client_copy = client.clone();
        let fn_name_copy = fn_name.clone();
        handles.push(tokio::spawn(async move{
            let response = client_copy.call_fn(&fn_name_copy, &("ru", "EUR", None::<String>)).await.unwrap();

            let res: (Vec<CountryData>, )= response.decode_single().unwrap();
            let v = COUNTER.fetch_add(1, Ordering::SeqCst);
            // println!("v={}",v);
            if v == count - 1 {
                println!("Test  finished! count={:?} res={:?}", v, res);
                let elapsed = start.elapsed();
                println!("time {:?}", elapsed);
                std::process::exit(0);
            }
            // Ok(())
        }));
        // threaded_rt.sh
    };

    futures::future::join_all(handles).await;
    // threaded_rt.shutdown_timeout(core::time::Duration::from_millis(100));
    Ok(())

}
