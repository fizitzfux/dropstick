use alloc::boxed::Box;

use crate::core1::ui::{menu::{settings_menu::SettingsMenu, song_menu::SongMenu, Menu}, MenuReturn};

pub struct MainMenu {
    selection: u8,
    max_selection: u8,
}

impl MainMenu {
    pub fn new() -> MainMenu {
        MainMenu { selection: 0, max_selection: 1 }
    }
}

impl Menu for MainMenu {
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
            0 => MenuReturn { push: Some(Box::new(SongMenu::new())), pop: false },
            1 => MenuReturn { push: Some(Box::new(SettingsMenu::new())), pop: false },
            _ => return MenuReturn::default(),
        }
    }
    
    fn get_menu_name(&self) -> &str {
        "Main Menu"
    }
    
    fn get_item(&self, index: u8) -> &str {
        match index {
            0 => "Songs",
            1 => "Settings",
            _ => "",
        }
    }

    fn get_selection_pointer(&self) -> u8 {
        self.selection
    }
}
