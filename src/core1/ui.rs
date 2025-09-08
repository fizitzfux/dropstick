use alloc::boxed::Box;
use defmt::{error, trace};

use crate::core1::{ui::menu::Menu, ButtonStates};

pub mod menu;

pub struct UI {
    menu_stack: [Option<Box<dyn Menu>>; 8],
    pointer: u8,
}

impl UI {
    pub fn new(root_menu: Box<dyn Menu>) -> UI {
        let mut menu_stack= [const { None }; 8];
        menu_stack[0] = Some(root_menu);
        UI { 
            menu_stack,
            pointer: 0,
        }
    }

    pub fn pop_menu(&mut self) {
        if self.pointer == 0 {
            error!("Tried to pop root menu!");
        }
        self.menu_stack[self.pointer as usize] = None;
        self.pointer -= 1;

        trace!("Entered menu: {}", self.get_current_menu().as_ref().unwrap().get_menu_name())
    }

    pub fn push_menu(&mut self, menu: Box<dyn Menu>) {
        trace!("Entered menu: {}", menu.get_menu_name());

        self.pointer += 1;
        self.menu_stack[self.pointer as usize] = Some(menu);
    }

    pub fn get_current_menu(&self) -> &Option<Box<dyn Menu>> {
        &self.menu_stack[self.pointer as usize]
    }

    pub fn check_button_input(&mut self, states: ButtonStates) {
        let menu = self.menu_stack[self.pointer as usize].as_mut().unwrap();
        
        if states.btn_1_pressed {
            trace!("btn 1 pressed");
            menu.forward();
        }
        if states.btn_2_pressed {
            trace!("btn 2 pressed");
            menu.back();
        }
        if states.btn_3_pressed {
            trace!("btn 3 pressed");
            let menu_return = menu.confirm();
            if menu_return.pop {
                self.pop_menu();
            }
            if let Some(new_menu) = menu_return.push {
                self.push_menu(new_menu);
            }
        }
    }
}

#[derive(Default)]
pub struct MenuReturn {
    push: Option<Box<dyn Menu>>,
    pop: bool,
}
