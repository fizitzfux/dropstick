use ssd1306::{mode::{BufferedGraphicsMode, DisplayConfig}, prelude::{DisplayRotation, WriteOnlyDataCommand}, size::DisplaySize128x32, Ssd1306};
use embedded_graphics::{mono_font::{ascii::FONT_6X10, iso_8859_1::FONT_4X6, MonoTextStyle}, pixelcolor::BinaryColor, prelude::Point, text::{Alignment, Text}};
use embedded_graphics::Drawable;

use crate::core1::ui::UI;

type DisplayType<DI> = Ssd1306<DI, ssd1306::prelude::DisplaySize128x32, BufferedGraphicsMode<ssd1306::prelude::DisplaySize128x32>>;

pub struct Screen<DI> {
    display: DisplayType<DI>,
}

impl<DI> Screen<DI> 
    where
    DI: WriteOnlyDataCommand,
{
    pub fn new(interface: DI) -> Screen<DI> {
        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
        display.init().unwrap();

        Screen { 
            display,
        }
    }

    pub fn draw_from_ui(&mut self, ui: &UI) {
        let menu = ui.get_current_menu().as_ref().unwrap();
        let selection = menu.get_selection_pointer();
        self.draw_vertical_menu(menu.get_menu_name(), (
            menu.get_item(selection - 1),
            menu.get_item(selection),
            menu.get_item(selection + 1)
        ));
    }

    fn draw_vertical_menu(&mut self, titel: &str, items: (&str, &str, &str)) {
        self.display.clear_buffer();
        let big_font_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let small_font_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);

        //TITLE
        Text::with_alignment(
        titel,
        Point::new(64, 6),
        big_font_style,
        Alignment::Center,
        )
        .draw(&mut self.display).unwrap();
        
        //Top
        Text::with_alignment(
        items.0,
        Point::new(0, 13),
        small_font_style,
        Alignment::Left,
        )
        .draw(&mut self.display).unwrap();

        //Center
        Text::with_alignment(
        items.1,
        Point::new(0, 22),
        big_font_style,
        Alignment::Left,
        )
        .draw(&mut self.display).unwrap();

        //Bottom
        Text::with_alignment(
        items.2,
        Point::new(0, 29),
        small_font_style,
        Alignment::Left,
        )
        .draw(&mut self.display).unwrap();

        self.display.flush().unwrap();
    }

}
