use core::cell::{OnceCell, UnsafeCell};

use critical_section::Mutex;
use defmt::{debug, error, info, trace};
use embedded_hal::digital::InputPin;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{Mode, SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use fugit::RateExtU32;
use rp2040_hal::{clocks::ClocksManager, gpio::{self, bank0::{Gpio2, Gpio3, Gpio4, Gpio5, Gpio7, Gpio8}, FunctionSio, PullUp, SioInput}, multicore::{Multicore, Stack}, pac::{self, interrupt, PPB, PSM, RESETS, SPI0}, sio::SioFifo, spi, Clock, Sio, Timer};

/// Allocate stack for the second core
static mut CORE1_STACK: Stack<4096> = Stack::new();



/// A dummy timesource, which is mostly important for creating files.
#[derive(Default)]
pub struct DummyTimesource();

impl TimeSource for DummyTimesource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/* SHARED WITH INTERRUPT */

struct InputPins {
    btn_1: gpio::Pin<Gpio7, FunctionSio<SioInput>, PullUp>,
    btn_2: gpio::Pin<Gpio8, FunctionSio<SioInput>, PullUp>,
}
#[derive(Default)]
struct ButtonStates {
    btn_1_pressed: bool,
    btn_2_pressed: bool,
}

// The INPUT_PINS that are shared with the interrupt routine.
static INPUT_PINS: Mutex<UnsafeCell<OnceCell<InputPins>>> = Mutex::new(UnsafeCell::new(OnceCell::new()));

/// Safely accesses global INPUT_PINS variable.
/// WARNING: Uses critical section.
fn access_input_pins<T: Fn(&mut InputPins, &mut ButtonStates) -> ()> (function: T) -> () {
    critical_section::with(|cs| {
        let input_pins_cell = INPUT_PINS.borrow(cs);
        let input_pins = unsafe {input_pins_cell.as_mut_unchecked()}.get_mut().unwrap();

        let button_states_cell = BUTTON_STATES.borrow(cs);
        let button_states = unsafe {button_states_cell.as_mut_unchecked()}.get_mut().unwrap();

        function(input_pins, button_states);
    });
}

/// Safely set global INPUT_PINS variable.
/// WARNING: Only call this *once*, else forced panic.
fn set_input_pins(input_pins: InputPins) -> () {
    critical_section::with(|cs| {
        let input_pins_cell = INPUT_PINS.borrow(cs);
        let result = unsafe {input_pins_cell.as_mut_unchecked()}.set(input_pins);
        if result.is_err() {
            error!("Shared INPUT_PINS Mutex failed to set!");
        };
    });
}

// The INPUT_PINS that are shared with the interrupt routine.
static BUTTON_STATES: Mutex<UnsafeCell<OnceCell<ButtonStates>>> = Mutex::new(UnsafeCell::new(OnceCell::new()));

/// Safely accesses global INPUT_PINS variable.
/// WARNING: Uses critical section.
fn access_button_states<T: Fn(&mut ButtonStates) -> ()> (function: T) -> () {
    critical_section::with(|cs| {
        let button_states_cell = BUTTON_STATES.borrow(cs);
        let button_states = unsafe {button_states_cell.as_mut_unchecked()}.get_mut().unwrap();

        function(button_states);
    });
}

/// Safely set global INPUT_PINS variable.
/// WARNING: Only call this *once*, else forced panic.
fn set_button_states(button_states: ButtonStates) -> () {
    critical_section::with(|cs| {
        let button_states_cell = BUTTON_STATES.borrow(cs);
        let result = unsafe {button_states_cell.as_mut_unchecked()}.set(button_states);
        if result.is_err() {
            error!("Shared INPUT_PINS Mutex failed to set!");
        };
    });
}

#[interrupt]
fn IO_IRQ_BANK0() {
    
    access_input_pins(|pins, states| {
        
        if pins.btn_1.is_low().ok().unwrap() {
            states.btn_1_pressed = true;
        }
        if pins.btn_2.is_low().ok().unwrap() {
            states.btn_2_pressed = true;
        }
        
        pins.btn_1.clear_interrupt(gpio::Interrupt::EdgeLow);
        pins.btn_2.clear_interrupt(gpio::Interrupt::EdgeLow);
    });
}


pub fn init(psm: &mut PSM, ppb: &mut PPB, fifo: &mut SioFifo, function: impl FnOnce() -> () + Send + 'static) -> () {
    let mut mc = Multicore::new(psm, ppb, fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

    trace!("Starting second core!");
    let _test = core1.spawn(
        #[allow(static_mut_refs)]
        unsafe {CORE1_STACK.take().unwrap()},
        function,
    ).expect("Failed to start second core!");
    trace!("Successfully started second core!");
}

pub fn main(
    resets: &mut RESETS,
    spi0: SPI0,
    clocks: ClocksManager,
    gpio2: gpio::Pin<Gpio2, gpio::FunctionNull, gpio::PullDown>,
    gpio3: gpio::Pin<Gpio3, gpio::FunctionNull, gpio::PullDown>,
    gpio4: gpio::Pin<Gpio4, gpio::FunctionNull, gpio::PullDown>,
    gpio5: gpio::Pin<Gpio5, gpio::FunctionNull, gpio::PullDown>,
    timer: Timer,
    gpio7: gpio::Pin<Gpio7, gpio::FunctionNull, gpio::PullDown>,
    gpio8: gpio::Pin<Gpio8, gpio::FunctionNull, gpio::PullDown>,
) -> ! {
    info!("Core 1 says hello! :3c");

    let mut inter_core_fifo = {
        let pac = unsafe {
            pac::Peripherals::steal()
        };
        let sio = Sio::new(pac.SIO);
        sio.fifo
    };

    // Set up our SPI pins into the correct mode
    let spi_sclk: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = gpio2.reconfigure();
    let spi_mosi: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = gpio3.reconfigure();
    let spi_miso: gpio::Pin<_, gpio::FunctionSpi, gpio::PullUp> = gpio4.reconfigure();
    let spi_cs = gpio5.into_push_pull_output();

    // Create the SPI driver instance for the SPI0 device
    let spi = spi::Spi::<_, _, _, 8>::new(spi0, (spi_mosi, spi_miso, spi_sclk));

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        resets,
        clocks.peripheral_clock.freq(),
        400_u32.kHz(), // card initialization happens at low baud rate
        embedded_hal::spi::MODE_0,
    );

    let spi_device = ExclusiveDevice::new(spi, spi_cs, timer).unwrap();

    trace!("Initialize SPI SD/MMC data structures...");
    let sdcard = SdCard::new(spi_device, timer);
    let volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());

    trace!("Init SD card controller...");
    
    // Now that the card is initialized, clock can go faster
    volume_mgr.device(|device| {
        device.spi(|spi| {
            spi.bus_mut().set_baudrate(clocks.peripheral_clock.freq(), 16_u32.MHz());
            DummyTimesource::default()
        })
    });
    info!("Initialized SD card.");
    
    trace!("Getting Volume 0...");
    let volume = volume_mgr.open_raw_volume(VolumeIdx(0)).expect("Failed!");

    let volume_name = volume_mgr.get_root_volume_label(volume).expect("Failed!").expect("Failed!");
    let name = str::from_utf8(volume_name.name()).expect("Failed!");
    trace!("Card name is \"{}\"", name);
    
        //Button Interrupt Setup
        let gpio7 = gpio7.into_pull_up_input();
        let gpio8 = gpio8.into_pull_up_input();
        gpio7.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);
        gpio8.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);
        set_button_states(ButtonStates::default());
        set_input_pins(InputPins { btn_1: gpio7, btn_2: gpio8 });
        unsafe {pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0)};

    // After we have the volume (partition) of the drive we got to open the
    // root directory:
    let dir = volume_mgr.open_root_dir(volume).expect("Failed!");

    // This shows how to iterate through the directory and how
    // to get the file names (and print them in hope they are UTF-8 compatible):
    volume_mgr.iterate_dir(dir, |file| {
        debug!(
            "/{}.{}",
            core::str::from_utf8(file.name.base_name()).unwrap(),
            core::str::from_utf8(file.name.extension()).unwrap()
        );
    }).unwrap();

    // Next we going to read a file from the SD card:
    if let Ok(file) = volume_mgr.open_file_in_dir(dir, "Daisies.wav", Mode::ReadOnly) {
        let mut read_bytes: usize = 0;
        loop {
            let mut buffer = [0u8; 128];
            let amount_read = volume_mgr.read(file, &mut buffer).unwrap();
            read_bytes += amount_read;
    
            for i in 0..amount_read {
                inter_core_fifo.write_blocking(buffer[i] as u32);
            }

            if amount_read < buffer.len() {
                break;
            }

            access_button_states(|states| {
                if states.btn_1_pressed {
                    info!("Button 1 pressed!");
                    states.btn_1_pressed = false;
                }
                if states.btn_2_pressed {
                    info!("Button 2 pressed!");
                    states.btn_2_pressed = false;
                }
            });

        }

        volume_mgr.close_file(file).unwrap();

        info!("Read {} bytes :3", read_bytes);
    }

    volume_mgr.free();

    loop {}
}
