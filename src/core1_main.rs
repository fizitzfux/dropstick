use defmt::info;
use rp2040_hal::{multicore::{Multicore, Stack}, pac::{PPB, PSM}, sio::SioFifo};

/// Allocate stack for the second core
static mut CORE1_STACK: Stack<4096> = Stack::new();


pub fn init(psm: &mut PSM, ppb: &mut PPB, fifo: &mut SioFifo, function: impl FnOnce() -> ! + Send + 'static) -> () {
    let mut mc = Multicore::new(psm, ppb, fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

    core1.spawn(
        #[allow(static_mut_refs)]
        unsafe {&mut CORE1_STACK.mem},
        function,
    ).ok();
}

pub fn main() -> ! {
    info!("Core 1 says hello! :3c");

    loop {}
}
