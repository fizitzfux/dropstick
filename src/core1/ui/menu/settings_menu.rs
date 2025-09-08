use defmt::info;
use crate::core1::ui::{menu::Menu, MenuReturn};


pub struct SettingsMenu {
    selection: u8,
    max_selection: u8,
}

impl SettingsMenu {
    pub fn new() -> SettingsMenu {
        SettingsMenu { selection: 0, max_selection: 1 }
    }
}

impl Menu for SettingsMenu {
    fn back(&mut self) {
        if self.selection == 0 {
            self.selection = self.max_selection;
        }else {
            self.selection -= 1;
        }
    }

    fn forward(&mut self) {
        if self.selection == self.max_selection {
            self.selection = 0;
        }else {
            self.selection += 1;
        }
    }

    fn confirm(&mut self) -> MenuReturn {
        match self.selection {
            0 => MenuReturn { push: None, pop: true },
            1 => {
                info!("Setting changed :3");
                MenuReturn::default()
            }
            _ => MenuReturn::default(),
        }
    }
    
    fn get_menu_name(&self) -> &str {
        "Settings"
    }
}
