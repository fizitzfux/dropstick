use core::cell::{OnceCell, UnsafeCell};
use cortex_m::prelude::_embedded_hal_PwmPin;
use critical_section::Mutex;
use defmt::{error, info, trace};
use embedded_hal::{digital::InputPin};
use rp2040_hal::{self as hal, gpio::{bank0::{Gpio16, Gpio6}, FunctionNull, Pin, PullDown}, pac::{self, interrupt}, pwm::{FreeRunning, Pwm0, Slice, Slices}, sio::SioFifo, Timer};

use crate::player::wav_streaming::WAVStreamPlayer;


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



pub fn main(gpio6: Pin<Gpio6, FunctionNull, PullDown>, gpio16: Pin<Gpio16, FunctionNull, PullDown>, timer: Timer, pwm_slices: Slices, inter_core_fifo: &mut SioFifo) -> ! {
    info!("Core 0 says hiii! X3");

    // Set up wav player
    let mut buf = [0; 128];
    let mut wav_player = WAVStreamPlayer::new(&mut buf);
    
    {
        // Get our audio PWM peripheral
        let mut pwm: Slice<Pwm0, FreeRunning> = pwm_slices.pwm0;

        // Let the player configure it
        wav_player.init(&mut pwm);
        
        // Set its output channel
        pwm.channel_a.output_to(gpio16);
        
        // Give it away to our shared Mutex for it,
        // so the interrupt handler can access it as well
        set_pwm(pwm);

        // Unmask the PWM_IRQ_WRAP interrupt so we start receiving events.
        unsafe {pac::NVIC::unmask(pac::Interrupt::PWM_IRQ_WRAP)};
    }


    // Pause button state
    let mut button_pin = gpio6.into_pull_up_input();
    let mut button_already_down: bool = button_pin.is_low().ok().expect("huh?? :0");

    // Player state
    let mut paused: bool = false;
    let mut start_time: u64 = timer.get_counter().ticks();

    // Playback loop
    loop {
        // Pause button
        if button_pin.is_low().is_ok_and(|val| val == true) {
            if !button_already_down {
                button_already_down = true;
                paused = !paused;
            }
        }
        else {
            button_already_down = false;
        }

        // Do not play if we are paused
        if paused {continue;}

        // Get sample
        let val = wav_player.get_next_sample();

        // Play sample
        access_pwm(|pwm| {
            pwm.channel_a.set_duty(val);
        });

        // Get more samples if we're out
        if wav_player.counter >= wav_player.current_buffer.len() -1 {
            // Read from inter-core fifo
            for i in 0..wav_player.current_buffer.len() {
                wav_player.current_buffer[i] = inter_core_fifo.read_blocking() as u8;
            };
            wav_player.counter = 0;

            // Log time it took to play last buffer for performance debugging
            let new_start_time = timer.get_counter().ticks();
            let current_time = timer.get_counter().ticks();
            trace!("Stream frame took: {}us goal: {}us", current_time - start_time, 31.25*wav_player.current_buffer.len() as f32);
            start_time = new_start_time;
        }
        // Else wait till we need to supply next sample
        else {
            wav_player.await_next_tick();
        }

        // Loop, so we play next sample
    }
}
