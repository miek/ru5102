extern crate ru5102;

fn main() {
    let mut reader = ru5102::Reader::new("/dev/ttyUSB0");
    loop {
        reader.inventory().unwrap();
    }
}
