use ravif::{Encoder, Img};
use rgb::FromSlice;

fn main() {
    let data = include_bytes!("./test.bin");
    let length = 36;

    let input = Img::new(data.as_rgba(), length, length);

    drop(Encoder::new().encode_rgba(input));
}
