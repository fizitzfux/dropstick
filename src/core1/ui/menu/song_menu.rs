use defmt::info;

use crate::core1::ui::{menu::Menu, MenuReturn};


pub struct SongMenu {
    selection: u8,
    max_selection: u8,
}

impl SongMenu {
    pub fn new() -> SongMenu {
        SongMenu { selection: 0, max_selection: 5 }
    }
}

impl Menu for SongMenu {
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
            1..=5 => {
                info!("played song {}", self.selection);
                MenuReturn::default()
            }
            _ => MenuReturn::default(),
        }
    }

    fn get_menu_name(&self) -> &str {
        "Song Menu"
    }

    fn get_item(&self, index: u8) -> &str {
        match index {
            0 => "Back",
            1 => "Play Song 1",
            2 => "Play Song 2",
            3 => "Play Song 3",
            4 => "Play Song 4",
            5 => "Play Song 5",
            _ => "",
        }
    }
    
    fn get_selection_pointer(&self) -> u8 {
        self.selection
    }
}
