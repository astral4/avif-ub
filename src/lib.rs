use core::hint::black_box;
use rav1e::{Config, Pixel};

pub fn run() {
    let x = black_box(true);

    if x {
        run_inner(0u8)
    } else {
        run_inner(0u16)
    }
}

fn run_inner<P: Pixel>(_: P) {
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
