//! The Main Application Library

mod console;
mod debug;
mod entity;
mod player_control;
mod rule;
mod scenario;
mod types;
mod world;

use crate::world::*;

/// Runs the program.
pub fn run() {
    // FIRST, Print the introduction
    print_introduction();

    // NEXT, create the game world.
    let mut the_world: World = scenario::build();
    let world = &mut the_world;

    player_control::describe_player_location(world, false);

    // NEXT, enter the game loop.
    loop {
        // FIRST, get the user's input
        let cmd = console::get_command(">");

        // NEXT, let the player do what he does.
        player_control::system(world, &cmd);

        // NEXT, handle rules
        rule::system(world);

        // NEXT, Increment the clock
        // TODO: Probably don't want to do this here.  Some commands should
        // take time, and some shouldn't.  This should probably be in the
        // player_control system.
        world.clock += 1;
    }
}

fn print_introduction() {
    println!("Welcome to Advent!\n");
}
