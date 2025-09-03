use defmt::info;
use embedded_hal::{digital::v2::InputPin, PwmPin};
use rp2040_hal::{self as hal, gpio::Pins, pac::{self, interrupt}, pwm::{FreeRunning, Pwm0, Slice, Slices}, sio::SioFifo, Timer};

use crate::player::wav::WAVPlayer;

/// The audio file to play.
/// Format: WAV 8-bit unsigned mono 8_000 Hz
const AUDIO: &[u8] = include_bytes!("sample.wav");


/* SHARED WITH INTERRUPT */

/// The hardware PWM driver that is shared with the interrupt routine.
static mut PWM: Option<hal::pwm::Slice<Pwm0, FreeRunning>> = None;

#[interrupt]
fn PWM_IRQ_WRAP() {
    // SAFETY: This is not used outside of interrupt critical sections in the main thread.
    let pwm = unsafe { PWM.as_mut() }.unwrap();

    // Clear the interrupt (so we don't immediately re-enter this routine)
    pwm.clear_interrupt();
}


pub fn main(pins: Pins, timer: Timer, pwm_slices: Slices, inter_core_fifo: &mut SioFifo) -> ! {
    let mut pwm: Slice<Pwm0, FreeRunning> = pwm_slices.pwm0;

    let mut wav_player = WAVPlayer::new(AUDIO);
    wav_player.init(&mut pwm);

    // Output channel A on PWM0 to GPIO16
    pwm.channel_a.output_to(pins.gpio16);

    unsafe {
        // Share the PWM with our interrupt routine.
        PWM = Some(pwm);

        // Unmask the PWM_IRQ_WRAP interrupt so we start receiving events.
        pac::NVIC::unmask(pac::Interrupt::PWM_IRQ_WRAP);
    }


    let button_pin = pins.gpio2.into_pull_up_input();
    let mut button_already_down: bool = button_pin.is_low().ok().expect("huh?? :0");


    let mut start_time: u64 = timer.get_counter().ticks();
    const EXPECTED: u64 = 1_000_000 / (2_u64.pow(15));
    loop {
        let val = wav_player.get_next_sample();
        let i = wav_player.get_current_sample();

        cortex_m::interrupt::free(|_| {
            // SAFETY: Interrupt cannot currently use this while we're in a critical section.
            let channel = &mut unsafe { PWM.as_mut() }.unwrap().channel_a;
            channel.set_duty(val);
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
