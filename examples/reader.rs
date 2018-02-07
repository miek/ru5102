extern crate ru5102;

fn main() {
    let mut reader = ru5102::Reader::new("/dev/ttyUSB0").unwrap();
    loop {
        reader.inventory().unwrap();
    }
}
