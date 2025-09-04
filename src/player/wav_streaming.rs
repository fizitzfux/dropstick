use rp2040_hal::pwm::{FreeRunning, Slice, SliceId};

/// Plays WAV 8 bit unsigned mono files at 32kHz
pub struct WAVStreamPlayer<'buf> {
    pub counter: usize,
    pub current_buffer: &'buf mut[u8],
}

impl WAVStreamPlayer<'_> {
    pub fn new<'buf>(buf: &'buf mut [u8]) -> WAVStreamPlayer<'buf> {
        WAVStreamPlayer {
            counter: 0,
            current_buffer: buf,
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
        let mut sample = self.counter + 1;
        self.counter = sample;
        if sample >= self.current_buffer.len() {
            sample = 0;
        }

        let raw_value = self.current_buffer[sample];

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
}
