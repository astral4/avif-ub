use core::hint::black_box;
use rav1e::{Config, Pixel};

pub fn run() {
    if black_box(true) {
        run_inner::<u8>();
    } else {
        run_inner::<u16>();
    }
}

fn run_inner<P: Pixel>() {
    rayon::join(
        || {},
        || {
            let mut ctx = Config::new().new_context::<P>().unwrap();
            let frame = ctx.new_frame();

            ctx.send_frame(frame).unwrap();
            ctx.flush();

            drop(ctx.receive_packet().unwrap());
        },
    );
}
