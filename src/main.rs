// Audio: https://pinoysa.us/codes/pico_audio.txt

#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(unsafe_cell_access)]

use embedded_alloc::Heap;
use rp2040_hal::{self as hal, pac, pll::common_configs::PLL_USB_48MHZ, Timer};

mod player;
mod clock_init;
mod core0_main;
mod core1_main;

// Formatting machinery
use defmt_rtt as _;
// Panicking machinery
use panic_probe as _;

/// Allocate bootloader
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// Allocate heap
#[global_allocator]
static mut HEAP: Heap = Heap::empty();


#[rp2040_hal::entry]
fn main() -> ! {
    // Take our peripherals
    let mut pac = pac::Peripherals::take().unwrap();
    // let core = pac::CorePeripherals::take().unwrap();

    // Initialize heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        #[allow(static_mut_refs)]
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    // Set up the watchdog
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = clock_init::init_system_clocks(
        clock_init::XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        clock_init::PLL_SYS_131MHZ,
        PLL_USB_48MHZ,
        &mut pac.RESETS,
        &mut watchdog,
    ).ok().unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let mut sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Init timer
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // Init PWMs
    let pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    core1_main::init(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo, move || {
        core1_main::main(
            &mut pac.RESETS,
            pac.SPI0,
            clocks,
            pins.gpio2,
            pins.gpio3,
            pins.gpio4,
            pins.gpio5,
            timer,
        )
    });

    core0_main::main(
        pins.gpio6,
        pins.gpio16,
        timer,
        pwm_slices,
        &mut sio.fifo,
    );
}
