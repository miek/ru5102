extern crate ru5102;

fn main() {
    let reader = ru5102::Reader::new("/dev/ttyUSB0");
    reader.inventory().unwrap();
}
