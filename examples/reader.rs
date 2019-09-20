extern crate ru5102;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut reader = ru5102::Reader::new(&args[1]).unwrap();
    println!("Reader info: {:?}", reader.reader_information().unwrap());
    loop {
        let inv = reader.inventory().unwrap();
        if inv.len() > 0 {
            for epc in inv.iter() {
                println!("Found tag: {:?}", epc);
                /*
                let read_cmd = ru5102::ReadCommand {
                    epc: epc.to_vec(),
                    location: ru5102::ReadLocation::TID,
                    start_address: 0,
                    count: 100,
                    password: None,
                    mask_address: None,
                    mask_length: None
                };
                let data = reader.read_data(read_cmd).unwrap();
                println!("Tag data: {:?}", data);
                */
                println!("");
            }
        }
    }
}
