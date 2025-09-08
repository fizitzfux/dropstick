use crate::core1::ui::MenuReturn;

pub mod main_menu;
pub mod song_menu;
pub mod settings_menu;

pub trait Menu {
    fn back(&mut self);
    fn forward(&mut self);
    fn confirm(&mut self) -> MenuReturn;

    fn get_menu_name(&self) -> &str;
}
