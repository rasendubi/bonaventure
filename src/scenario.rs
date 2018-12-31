//! Scenario definition

use crate::types::Event;
use crate::entity::ID;
use crate::phys;
use crate::types::Action::*;
use crate::types::Dir::*;
use crate::types::Event::*;
use crate::types::Flag;
use crate::types::Flag::*;
use crate::types::ProseType::*;
use crate::visual::Buffer;
use crate::world::World;

// Constant entity tags, for lookup
const NOTE: &str = "note";

// User-defined flags
// TODO: These constants should only be used in the scenario itself; but at present they
// are still used by the "wash hands" command code in player_control.rs.  Once that's
// implemented clearly, they should no longer be "pub".
const DIRTY: Flag = User("DIRTY");
pub const DIRTY_HANDS: Flag = User("DIRTY_HANDS");
pub const HAS_WATER: Flag = User("HAS_WATER");

/// Build the initial state of the game world.
pub fn build() -> World {
    // FIRST, make the empty world
    let mut the_world = World::new();
    let world = &mut the_world;

    // // NEXT, Make the player
    world.pid = world
        .add("self")
        .player()
        .prose_hook(Thing, &|world, id| player_visual(world, id))
        .flag(DIRTY_HANDS)
        .id();

    let pid = world.pid;

    // NEXT, make the rooms.

    // Room: Clearing
    let clearing = world
        .add("clearing")
        .room("Clearing")
        .prose(Room, "A wide spot in the woods.  You can go east.")
        .id();

    // Room: Trail
    let trail = world
        .add("trail")
        .room("Trail")
        .prose(
            Room,
            "A trail from hither to yon.  You can go east or west.",
        )
        .id();

    // Room: Bridge
    let bridge = world
        .add("bridge")
        .room("Bridge")
        .prose(
            Room,
            "The trail crosses a small stream here.  You can go east or west.",
        )
        .flag(HAS_WATER)
        .id();

    world
        .add("stream")
        .thing("stream", "stream")
        .prose(
            Thing,
            "\
The stream comes from the north, down a little waterfall, and runs
away under the bridge.  It looks surprisingly deep, considering
how narrow it is.
        ",
        )
        .flag(Scenery)
        .put_in(bridge)
        .id();

    // Links
    world.twoway(clearing, East, West, trail);
    world.twoway(trail, East, West, bridge);

    // The note
    let note = world
        .add(NOTE)
        .thing("note", "note")
        .prose_hook(Thing, &|world, id| note_thing_prose(world, id))
        .prose_hook(Book, &|world, id| note_book_prose(world, id))
        .put_in(clearing)
        .id();

    world
        .add("rule-dirty-note")
        .always(GetThing(pid, note), &|w, e| predicate_rule_dirty_note(w, e))
        .action(Print(
            "The dirt from your hands got all over the note.".into(),
        ))
        .action(SetFlag(note, DIRTY));

    // The sword
    let sword = world
        .add("sword")
        .thing("sword", "sword")
        .prose(
            Thing,
            "\
The sword, if you want to call it that, is a three-foot length of dark hardwood
with a sharkskin hilt on one end.  It's polished so that it gleams, and it has no
sharp edges anywhere.  Carved along the length of it are the words
\"Emotional Support Sword (TM)\".
        ",
        )
        .put_in(trail)
        .id();

    world
        .add("rule-sword-get")
        .guard(GetThing(pid, sword), &|w,_| {
            !w.has_flag(w.pid, DIRTY_HANDS)
        })
        .action(Print(
            "\
Oh, you so didn't want to touch the sword with dirty hands.
Only the pure may touch this sword.
            "
            .into(),
        ))
        .action(Kill(pid));

    // Stories: Rules that supply backstory to the player.
    world
        .add("rule-story-1")
        .once(Turn, &|w,_| w.clock == 0)
        .action(Print(
            "\
You don't know where you are.  You don't even know where you want to
be.  All you know is that your feet are wet, your hands are dirty,
and gosh, this doesn't look anything like the toy aisle.
        "
            .into(),
        ));

    world
        .add("fairy-godmother-rule")
        .always(Turn, &|w, _| w.has_flag(w.pid, Dead))
        .action(Print(
            "\
A fairy godmother hovers over your limp body.  She frowns;
then, apparently against her better judgment, she waves
her wand.  There's a flash, and she disappears.
            "
            .into(),
        ))
        .action(Revive(pid));

    // NEXT, set the starting location.
    phys::put_in(world, world.pid, clearing);
    world.set_flag(world.pid, Seen(clearing));

    // NEXT, return the world.
    the_world
}

/// Returns the player's current appearance.
fn player_visual(world: &World, pid: ID) -> String {
    Buffer::new()
        .add("You've got all the usual bits.")
        .when(
            world.has_flag(pid, DIRTY_HANDS),
            "Your hands are kind of dirty, though.",
        )
        .when(
            !world.has_flag(pid, DIRTY_HANDS),
            "Plus, they're clean bits!",
        )
        .get()
}

/// Predicate for rule-note-dirty
fn predicate_rule_dirty_note(world: &World, event: &Event) -> bool {
    match event {
        GetThing(pid,note) => {
            world.has_flag(*pid, DIRTY_HANDS) && !world.has_flag(*note, DIRTY)
        }
        _ => false,
    }
}

fn note_thing_prose(world: &World, id: ID) -> String {
    if world.has_flag(id, DIRTY) {
        "A note, on plain paper.  It looks pretty grubby; someone's been mishandling it.".into()
    } else {
        "A note, on plain paper".into()
    }
}

fn note_book_prose(world: &World, id: ID) -> String {
    if world.has_flag(id, DIRTY) {
        "You've gotten it too dirty to read.".into()
    } else {
        "\
Welcome, dear friend.  Your mission, should you choose to
accept it, is to figure out how to get to the end of
the trail.  You've already taken the first big
step!
         "
        .into()
    }
}
