use fugit::RateExtU32;
use rp2040_hal::{self as hal, clocks::{ClocksManager, InitError}, pac, pll::PLLConfig, Watchdog};


/// The frequency of the on-board crystal
pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// This clock rate is closest to 176,400,000 Hz, which is a multiple of 44,100 Hz.
#[allow(dead_code)]
pub const PLL_SYS_176MHZ: PLLConfig = PLLConfig {
    vco_freq: fugit::Rate::<u32, 1, 1>::MHz(528),
    refdiv: 1,
    post_div1: 3,
    post_div2: 1,
};

/// This clock rate is closest to 131,072,000 Hz, which is a multiple of 32,000 Hz (the audio sample rate).
#[allow(dead_code)]
pub const PLL_SYS_131MHZ: PLLConfig = PLLConfig {
    vco_freq: fugit::Rate::<u32, 1, 1>::MHz(1572),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

/// Initialize system clocks and PLLs according to specified configs
#[allow(clippy::too_many_arguments)]
pub fn init_system_clocks(
    xosc_crystal_freq: u32,
    xosc_dev: pac::XOSC,
    clocks_dev: pac::CLOCKS,
    pll_sys_dev: pac::PLL_SYS,
    pll_usb_dev: pac::PLL_USB,
    pll_sys_cfg: PLLConfig,
    pll_usb_cfg: PLLConfig,
    resets: &mut pac::RESETS,
    watchdog: &mut Watchdog,
) -> Result<ClocksManager, InitError> {
    let xosc = hal::xosc::setup_xosc_blocking(xosc_dev, xosc_crystal_freq.Hz())
        .map_err(InitError::XoscErr)?;

    // Configure watchdog tick generation to tick over every microsecond
    watchdog.enable_tick_generation((xosc_crystal_freq / 1_000_000) as u8);

    let mut clocks = ClocksManager::new(clocks_dev);

    let pll_sys = hal::pll::setup_pll_blocking(
        pll_sys_dev,
        xosc.operating_frequency().into(),
        pll_sys_cfg,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;
    let pll_usb = hal::pll::setup_pll_blocking(
        pll_usb_dev,
        xosc.operating_frequency().into(),
        pll_usb_cfg,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;

    clocks
        .init_default(&xosc, &pll_sys, &pll_usb)
        .map_err(InitError::ClockError)?;
    Ok(clocks)
}
