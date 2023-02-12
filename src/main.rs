#![no_main]
#![no_std]

// Sample app that reads a bunch of Adafruit Clue sensors
// and displays them on the screen. Modify as needed.


use adafruit_clue::{Board, TFT};
use cortex_m_rt;
use embedded_hal::blocking::delay::DelayMs;
use nrf52840_hal::{Delay, Timer};

// CO2
use scd4x::scd4x::Scd4x;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::{
    mono_font::{ascii::FONT_7X13, MonoTextStyle},
    text::Text,
};
use shared_bus;
use st7789::{Orientation, ST7789};

use core::fmt::Write;
use heapless::String;

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut b = Board::take().unwrap();
    let sensor_i2c = b.sensors_i2c.twim(b.TWIM1);
    let shared_sensor_i2c = shared_bus::BusManagerSimple::new(sensor_i2c);
    let co2_timer = Timer::new(b.TIMER3);
    let mut co2 = Scd4x::new(shared_sensor_i2c.acquire_i2c(),co2_timer);
    co2.wake_up();
    co2.stop_periodic_measurement().unwrap();
    co2.reinit().unwrap();
    co2.start_periodic_measurement().unwrap();

    // TFT SPI
    b.tft.backlight_on();
    let tft_display_interface =
        SPIInterfaceNoCS::new(b.tft.spim(b.SPIM0), b.tft.dc.take().unwrap());
    let mut display = ST7789::new(
        tft_display_interface,
        b.tft.reset.take().unwrap(),
        TFT::XSIZE,
        TFT::YSIZE,
    );
    let mut delay = Delay::new(b.core_peripherals.SYST);
    display.init(&mut delay).unwrap();
    display.set_orientation(Orientation::Landscape).unwrap();
    display.clear(Rgb565::BLACK).unwrap();
    let mut timer = Timer::new(b.TIMER4);
    let clear_text_rect = |x, y| {
        Rectangle::new(Point::new(x, y), Size::new(240, 20))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    };

    loop {
        timer.delay_ms(5000 as u32);
        let co2_data = co2.measurement().unwrap();
        let mut co2_data_string: String<64> = String::new();
        write!(co2_data_string, "{0} PPM, {1:#.2} C, {2:#.2} RH",
               co2_data.co2, co2_data.temperature, co2_data.humidity).unwrap();
        clear_text_rect(0, 220).draw(&mut display).unwrap();
        text(0, 220, &co2_data_string).draw(&mut display).unwrap();
    }
}

fn text(x: i32, y: i32, s: &str) -> Text<MonoTextStyle<Rgb565>> {
    let text_style = MonoTextStyle::new(&FONT_7X13, Rgb565::WHITE);
    Text::new(s, Point::new(x + 10, y + 10), text_style)
}

#[panic_handler] // panicking behavior
unsafe fn panic(_pinfo: &core::panic::PanicInfo) -> ! {
    let mut b: Board = Board::steal();
    let mut timer = Timer::new(b.TIMER3);
    loop {
        b.leds.white.on();
        timer.delay_ms(500 as u32);
        b.leds.white.off();
        timer.delay_ms(100 as u32);
        //cortex_m::asm::bkpt();
    }
}
