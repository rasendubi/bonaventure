//! The Player Control System

use crate::script::Script;
use self::Status::*;
use crate::command;
use crate::command::Command;
use crate::debug;
use crate::entity::ID;
use crate::phys;
use crate::types::Dir::*;
use crate::types::Flag::*;
use crate::types::*;
use crate::visual;
use crate::world::*;
use crate::Game;
use std::collections::BTreeSet;

/// A status result.  Indicates the general category of the change.
#[derive(Copy, Clone, Debug)]
enum Status {
    /// Normal response: the world has been updated, and the change can be undone.
    Normal,

    /// Restart response; the game should be restarted from scratch.
    Restart,

    /// Undo the last command (plus anything that happened after, e.g., rule firings)
    Undo,
}

/// A status result, used for special commands
type StatusResult = Result<Status, String>;

/// Player Context: ID and initial location.
struct Player {
    pub id: ID,
    pub loc: ID,
}

/// The Player Control system.  Processes player commands.
pub fn system(game: &mut Game, input: &str) {
    // FIRST, get the current game state, for later undo.
    let undo_info = game.world.clone();

    // NEXT, get the player's context
    let player = Player {
        id: game.world.pid,
        loc: phys::loc(&game.world, game.world.pid),
    };

    // NEXT, handle the input
    let result = handle_input(game, &player, input);
    match result {
        Err(msg) => visual::error(&msg),
        Ok(Normal) => {
            game.save_for_undo(undo_info);
        }
        Ok(Restart) => game.restart(),
        Ok(Undo) => game.undo(),
    }
}

fn handle_input(game: &mut Game, player: &Player, input: &str) -> StatusResult {
    // FIRST, parse the input.
    let cmd = command::parse(&game.world, input)?;

    if cmd.is_debug {
        handle_debug_command(game, player, &cmd)
    } else {
        handle_normal_command(game, player, &cmd)
    }
}

fn handle_normal_command(game: &mut Game, player: &Player, cmd: &Command) -> StatusResult {
    let world = &mut game.world;
    let the_words: Vec<&str> = cmd.words.iter().map(|s| s.as_ref()).collect();
    let words = the_words.as_slice();

    // FIRST, handle custom commands.
    // TODO: Possible better way to handle this: write function that matches the words
    // and clones the specific handler.
    for handler in world.command_handlers.clone() {
        if handler.matches(words) {
            handler.execute(&mut game.world, player, words)?;
            return Ok(Normal);
        }
    }

    // NEXT, handle built-in commands
    match words {
        ["go", "north"] => cmd_go(world, player, North),
        ["north"] => cmd_go(world, player, North),
        ["go", "south"] => cmd_go(world, player, South),
        ["south"] => cmd_go(world, player, South),
        ["go", "east"] => cmd_go(world, player, East),
        ["east"] => cmd_go(world, player, East),
        ["go", "west"] => cmd_go(world, player, West),
        ["west"] => cmd_go(world, player, West),
        ["help"] => cmd_help(),
        ["look"] => cmd_look(world, player),
        ["inventory"] => cmd_inventory(world, player),
        ["examine", name] => cmd_examine(world, player, name),
        ["read", name] => cmd_read(world, player, name),
        ["get", name] => cmd_get(world, player, name),
        ["pick", "up", name] => cmd_get(world, player, name),
        ["drop", name] => cmd_drop(world, player, name),
        ["undo"] => cmd_undo(game),
        ["restart"] => cmd_restart(),
        ["quit"] => cmd_quit(),

        // Error
        _ => Err("I don't understand.".into()),
    }
}

// User Commands


/// Display basic help, i.e., what commands are available.
fn cmd_help() -> StatusResult {
    visual::info(
        "\
You've got the usual commands: n, s, e, w, look, get, drop, quit.
You know.  Like that.
    ",
    );

    Ok(Normal)
}

/// Move the player in the given direction
fn cmd_go(world: &mut World, player: &Player, dir: Dir) -> StatusResult {
    match phys::follow_link(world, player.loc, dir) {
        Some(LinkDest::Room(dest)) => {
            phys::enter_room(world, player.id, dest)?;
            Ok(Normal)
        },
        Some(LinkDest::DeadEnd(prose)) => {
            visual::info(&prose);
            Ok(Normal)
        }
        None => {
            Err("You can't go that way.".into())
        }
    }
}

/// Re-describe the current location.
fn cmd_look(world: &World, player: &Player) -> StatusResult {
    visual::room(world, player.loc);
    Ok(Normal)
}

/// Display the player's inventory.
fn cmd_inventory(world: &World, player: &Player) -> StatusResult {
    visual::player_inventory(world, player.id);
    Ok(Normal)
}

/// Describe a thing in the current location.
fn cmd_examine(world: &World, player: &Player, name: &str) -> StatusResult {
    if let Some(thing) = find_noun(world, phys::visible(world, player.id), name) {
        if thing == player.id {
            visual::player(world, player.id);
        } else {
            visual::thing(world, thing);
        }
        Ok(Normal)
    } else {
        Err("You don't see any such thing.".into())
    }
}

/// Read a thing in the current location.
fn cmd_read(world: &mut World, player: &Player, name: &str) -> StatusResult {
    if let Some(thing) = find_noun(world, phys::visible(world, player.id), name) {
        // If it has no prose, it can't be read
        if !visual::can_read(world, thing) {
            return Err("You can't read that.".into());
        }

        // If he's holding it, or it's immovable, then he can read it.
        if phys::owns(world, player.id, thing) || world.has_flag(thing, Immovable) {
            phys::read_thing(world, player.id, thing)?;
            Ok(Normal)
        } else {
            Err("You don't have it.".into())
        }
    } else {
        // It isn't here.
        Err("You don't see any such thing.".into())
    }
}

/// Gets a thing from the location's inventory.
fn cmd_get(world: &mut World, player: &Player, noun: &str) -> StatusResult {
    // Does he already have it?
    if find_noun(world, phys::contents(world, player.id), noun).is_some() {
        return Err("You already have that.".into());
    }

    if find_noun(world, phys::immovable(world, player.loc), noun).is_some() {
        return Err("You can't take that!".into());
    }

    if let Some(thing) = find_noun(world, phys::gettable(world, player.id), noun) {
        // Get the thing.
        phys::get_thing(world, player.id, thing)?;
        return Ok(Normal);
    }

    Err("You don't see any such thing.".into())
}

/// Drops a thing you're carrying
fn cmd_drop(world: &mut World, player: &Player, noun: &str) -> StatusResult {
    if let Some(thing) = find_noun(world, phys::droppable(world, player.id), noun) {
        // Drop the thing
        phys::put_in(world, thing, player.loc);
        visual::act("Dropped.");
        Ok(Normal)
    } else if find_noun(world, phys::scenery(world, player.id), noun).is_some() {
        Err("You can't drop that!".into())
    } else if find_noun(world, phys::visible(world, player.id), noun).is_some() {
        Err("You aren't carrying that.".into())
    } else {
        Err("You don't see any such thing.".into())
    }
}

/// Undo the last command the game
fn cmd_undo(game: &mut Game) -> StatusResult {
    if game.has_undo() {
        visual::act("Undone.");
        Ok(Undo)
    } else {
        Err("Nothing to undo.".into())
    }
}

/// Restart the game
fn cmd_restart() -> StatusResult {
    visual::act("Restarting...");
    Ok(Restart)
}

/// Quit the game.
fn cmd_quit() -> StatusResult {
    visual::act("Bye, then.");
    ::std::process::exit(0);
}

//------------------------------------------------------------------------------
// Debugging commands

/// Handle debugging commands.
fn handle_debug_command(game: &mut Game, player: &Player, cmd: &Command) -> StatusResult {
    let words: Vec<&str> = cmd.words.iter().map(|s| s.as_ref()).collect();
    let world = &mut game.world;

    match words.as_slice() {
        ["list"] => cmd_debug_list(world),
        ["dump", id_arg] => cmd_debug_dump(world, id_arg),
        ["look", id_arg] => cmd_debug_look(world, id_arg),
        ["examine", id_arg] => cmd_debug_examine(world, id_arg),
        ["go", id_arg] => cmd_debug_go(world, player, id_arg),

        // Error
        _ => Err("I don't understand.".into()),
    }
}

/// List all of the available entities.
fn cmd_debug_list(world: &World) -> StatusResult {
    debug::list_world(world);
    Ok(Normal)
}

/// Dump information about the given entity, provided the ID string is valid.
fn cmd_debug_dump(world: &World, id_arg: &str) -> StatusResult {
    let id = parse_id(world, id_arg)?;
    debug::dump_entity(world, id);
    Ok(Normal)
}

/// Describe the room as though the player were in it.
fn cmd_debug_look(world: &World, id_arg: &str) -> StatusResult {
    let id = parse_id(world, id_arg)?;
    if world.is_room(id) {
        visual::room(world, id);
        Ok(Normal)
    } else {
        Err(format!("Entity {} is not a room.", id))
    }
}

/// Examine the thing fully, as though the player could see it.
fn cmd_debug_examine(world: &World, id_arg: &str) -> StatusResult {
    let id = parse_id(world, id_arg)?;
    if world.is_thing(id) {
        visual::thing(world, id);
        Ok(Normal)
    } else {
        Err(format!("Entity {} is not a thing.", id))
    }
}

/// Take the player to the room.
fn cmd_debug_go(world: &mut World, player: &Player, id_arg: &str) -> StatusResult {
    let loc = parse_id(world, id_arg)?;
    if world.is_room(loc) {
        phys::put_in(world, player.id, loc);
        visual::room(world, loc);
        Ok(Normal)
    } else {
        Err(format!("Entity {} is not a room.", loc))
    }
}

/// Parse a token as an entity tag or ID, return an ID on success and
/// an error result on failure.
fn parse_id(world: &World, token: &str) -> Result<ID, String> {
    // FIRST, is the token a tag?
    if let Some(id) = world.lookup_id(token) {
        return Ok(id);
    }

    // NEXT, is it an explicit ID?
    let id = match token.parse() {
        Ok(id) => id,
        Err(_) => {
            return Err(format!("Not an ID: {}", token));
        }
    };

    if !world.tags.contains_key(&id) {
        return Err(format!("Not an ID: {}", token));
    }

    Ok(id)
}

//-------------------------------------------------------------------------
// Parsing Tools

/// Finds a noun in the list of things.
fn find_noun(world: &World, ids: BTreeSet<ID>, noun: &str) -> Option<ID> {
    for id in ids {
        let thingc = &world.things[&id];
        if thingc.noun == noun {
            return Some(id);
        }
    }

    None
}

//-------------------------------------------------------------------------
// Command Handler

#[derive(Debug,Clone)]
enum CommandPattern {
    /// Match any single arbitrary word
    Verb(String),

    /// Match any pair of arbitrary words
    VerbNoun(String,String),

    /// Match any arbitrary word with a visible thing
    VerbVisible(String),
}

#[derive(Clone)]
pub struct CommandHandler {
    pattern: CommandPattern,
    hook: CommandHook,
}

impl CommandHandler {
    pub fn verb(word: &str, hook: CommandHook) -> Self {
        Self {
            pattern: CommandPattern::Verb(word.into()),
            hook
        }
    }

    pub fn verb_noun(word1: &str, word2: &str, hook: CommandHook) -> Self {
        Self {
            pattern: CommandPattern::VerbNoun(word1.into(), word2.into()),
            hook
        }
    }

    pub fn verb_visible(word: &str, hook: CommandHook) -> Self {
        Self {
            pattern: CommandPattern::VerbVisible(word.into()),
            hook
        }
    }

    /// Determines whether the words match the command's pattern.
    fn matches(&self, words: &[&str]) -> bool {
        match &self.pattern {
            CommandPattern::Verb(verb) => words.len() == 1 && words[0] == verb,
            CommandPattern::VerbNoun(verb,noun) => words.len() == 2 && words[0] == verb && words[1] == noun,
            CommandPattern::VerbVisible(verb) => words.len() == 2 && words[0] == verb,
        }
    }

    /// Executes the command
    fn execute(&self, world: &mut World, player: &Player, words: &[&str]) -> CommandResult {
        // FIRST, do special checks
        match &self.pattern {
            CommandPattern::VerbVisible(_) => {
                if find_noun(world, phys::visible(world, player.id), words[1]).is_none() {
                    return Err("You don't see any such thing.".into());
                }
            }
            _ => ()
        }

        // NEXT, compute the script, returning any error message
        let script = &mut Script::new();
        (self.hook)(world, words, script)?;

        // NEXT, execute the script
        script.execute(world);

        Ok(())
    }
}
