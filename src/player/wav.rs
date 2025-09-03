use core::{iter::Cycle, ops::Range};

use rp2040_hal::pwm::{FreeRunning, Slice, SliceId};

pub struct WAVPlayer<'buf> {
    counter: Cycle<Range<usize>>,
    current_sample: usize,
    buffer: &'buf[u8],
}

impl WAVPlayer<'_> {
    pub fn new<'buf>(buffer: &'buf[u8]) -> WAVPlayer<'buf> {
        WAVPlayer {
            counter: (0x2C..buffer.len()).cycle().into_iter(),
            current_sample: 0,
            buffer,
        }
    }

    pub fn init<PWM: SliceId>(&self, pwm: &mut Slice<PWM, FreeRunning>) -> () {
        pwm.default_config();
    
        // 131,000,000 Hz divided by (top * div.int).
        //
        // fPWM = fSYS / ((TOP + 1) * (CSR_PH_CORRECT + 1) * (DIV_INT + (DIV_FRAC / 16)))
        //
        // 32kHz ~= 131,000,000 / ((4096 + 1) * 1 * 1)
        const TOP: u16 = 4096;
        pwm.set_top(TOP);
        pwm.set_div_int(1);
    
        pwm.enable_interrupt();
        pwm.enable();
    }
    
    pub fn get_next_sample(&mut self) -> u16 {
        let sample = self.counter.next().unwrap();
        self.current_sample = sample;

        let raw_value = self.buffer[sample];

        // Rescale from unsigned u8 numbers to 0..4096 (the TOP register we specified earlier)
        //
        // The PWM channel will increment an internal counter register, and if the counter is
        // above or equal to this number, the PWM will output a logic high signal.
        let mut value = ((raw_value as u16) << 4) & 0xFFF;

        // Half value to reduce loudness
        value = value >> 1;

        value
    }
    
    pub fn await_next_tick(&self) -> () {
        // Throttle until the PWM channel delivers us an interrupt saying it's done
        // with this cycle (the internal counter wrapped). The interrupt handler will
        // clear the interrupt and we'll send out the next sample.
        cortex_m::asm::wfi();
    }

    pub fn get_current_sample(&self) -> usize {
        self.current_sample
    }

    pub fn reset(&mut self) -> () {
        self.counter.find(|&x| x == 0x2C);
    }
}
