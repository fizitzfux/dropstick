use core::cell::{OnceCell, UnsafeCell};
use critical_section::Mutex;
use defmt::{error, info};
use embedded_hal::{digital::v2::InputPin, PwmPin};
use rp2040_hal::{self as hal, gpio::Pins, pac::{self, interrupt}, pwm::{FreeRunning, Pwm0, Slice, Slices}, sio::SioFifo, Timer};

use crate::player::wav::WAVPlayer;

/// The audio file to play.
/// Format: WAV 8-bit unsigned mono 8_000 Hz
const AUDIO: &[u8] = include_bytes!("sample.wav");


/* SHARED WITH INTERRUPT */

// The hardware PWM driver that is shared with the interrupt routine.
static PWM: Mutex<UnsafeCell<OnceCell<hal::pwm::Slice<Pwm0, FreeRunning>>>> = Mutex::new(UnsafeCell::new(OnceCell::new()));

/// Safely accesses global PWM variable.
/// WARNING: Uses critical section.
fn access_pwm<T: Fn(&mut hal::pwm::Slice<Pwm0, FreeRunning>) -> ()> (function: T) -> () {
    critical_section::with(|cs| {
        let pwm_cell = PWM.borrow(cs);
        let pwm = unsafe {pwm_cell.as_mut_unchecked()}.get_mut().unwrap();

        function(pwm);
    });
}

/// Safely set global PWM variable.
/// WARNING: Only call this *once*, else forced panic.
fn set_pwm(pwm: hal::pwm::Slice<Pwm0, FreeRunning>) -> () {
    critical_section::with(|cs| {
        let pwm_cell = PWM.borrow(cs);
        let result = unsafe {pwm_cell.as_mut_unchecked()}.set(pwm);
        if result.is_err() {
            error!("Shared PWM Mutex failed to set!");
        };
    });
}


#[interrupt]
fn PWM_IRQ_WRAP() {
    access_pwm(|pwm| {
        // Clear the interrupt so we don't immediately re-enter this routine
        pwm.clear_interrupt();
    });
}



pub fn main(pins: Pins, timer: Timer, pwm_slices: Slices, inter_core_fifo: &mut SioFifo) -> ! {
    let mut wav_player = WAVPlayer::new(AUDIO);
    
    {
        // Get our audio PWM peripheral
        let mut pwm: Slice<Pwm0, FreeRunning> = pwm_slices.pwm0;

        // Let the player configure it
        wav_player.init(&mut pwm);
        
        // Set its output channel
        pwm.channel_a.output_to(pins.gpio16);
        
        // Give it away to our shared Mutex for it,
        // so the interrupt handler can access it as well
        set_pwm(pwm);

        // Unmask the PWM_IRQ_WRAP interrupt so we start receiving events.
        unsafe {pac::NVIC::unmask(pac::Interrupt::PWM_IRQ_WRAP)};
    }


    let button_pin = pins.gpio2.into_pull_up_input();
    let mut button_already_down: bool = button_pin.is_low().ok().expect("huh?? :0");


    let mut start_time: u64 = timer.get_counter().ticks();
    const EXPECTED: u64 = 1_000_000 / (2_u64.pow(15));
    loop {
        let val = wav_player.get_next_sample();
        let i = wav_player.get_current_sample();

        access_pwm(|pwm| {
            pwm.channel_a.set_duty(val);
        });

        if inter_core_fifo.is_write_ready() {
            inter_core_fifo.write(0);
        }

        if (i & 0b1111_1111) == 255 {
            let current_time = timer.get_counter().ticks();
            info!("DURATION: {}us MAX: {}us", current_time - start_time, EXPECTED);
        }

        wav_player.await_next_tick();

        if (i & 0b1111_1111) == 254 {
            start_time = timer.get_counter().ticks();
        }

        if button_pin.is_low().is_ok_and(|val| val == true) {
            if !button_already_down {
                button_already_down = true;
                wav_player.reset();
            }
        }
        else {
            button_already_down = false;
        }
    }
}
