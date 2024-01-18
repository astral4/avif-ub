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
            ctx.send_frame(ctx.new_frame()).unwrap();
            ctx.flush();
            // segfault occcurs in rav1e::Context::receive_packet()
            drop(ctx.receive_packet().unwrap());
        },
    );
}
