//! Scenario definition

use crate::types::Dir::*;
use crate::types::Var::*;
use crate::types::*;
use crate::world::*;

/// Build the initial state of the game world.
pub fn build() -> World {
    // FIRST, make the empty world
    let mut the_world = World::new();
    let world = &mut the_world;

    // NEXT, make the rooms.

    // Rooms
    let clearing = make_room(
        world,
        "clearing-1",
        "Clearing",
        "A wide spot in the woods.  You can go east.",
    );
    let trail = make_room(
        world,
        "trail-1",
        "Trail",
        "A trail from hither to yon.  You can go east or west.",
    );
    let bridge = make_room(
        world,
        "bridge-1",
        "Bridge",
        "\
The trail crosses a small stream here.  You can go east or west.
        "
    );
    world.set_var(bridge, HasWater);

    // Links
    connect(world, East, clearing, West, trail);
    connect(world, East, trail, West, bridge);

    // NEXT, make the things
    let note = make_thing(world, "note-1", "note", "It's illegible.");
    put_in(world, note, clearing);

    make_scenery(
        world,
        bridge,
        "stream-1",
        "stream",
        "\
The stream comes from the north, down a little waterfall, and runs
away under the bridge.  It looks surprisingly deep, considering
how narrow it is.
        ",
    );

    // Stories: Rules that supply backstory to the player.
    make_story(
        world,
        "story-1",
        &|world| world.clock == 2,
        "\
You don't know where you are.  You don't even know where you want to
be.  All you know is that your feet are wet, your hands are dirty,
and gosh, this doesn't look anything like the toy aisle.
    ",
    );

    // NEXT, Make the player
    make_player(world, clearing);

    // NEXT, return the world.
    the_world
}

/// Initializes the player's details
fn make_player(world: &mut World, start: ID) {
    world.pid = world
        .make("self")
        .name("self")
        .prose("You've got all the usual bits.")
        .location(start)
        .inventory()
        .var(DirtyHands)
        .var(Seen(start))
        .build();
}

/// Makes a room with the given name and prose, and an empty set of links.
/// Returns the room's ID.
fn make_room(world: &mut World, tag: &str, name: &str, text: &str) -> ID {
    world
        .make(tag)
        .name(name)
        .prose(text)
        .links()
        .inventory()
        .vars()
        .build()
}

/// Makes a portable object, and returns its ID.
fn make_thing(world: &mut World, tag: &str, name: &str, text: &str) -> ID {
    world
        .make(tag)
        .name(name)
        .prose(text)
        .vars()
        .build()
}

/// Makes a scenery object, and returns its ID.
fn make_scenery(world: &mut World, loc: ID, tag: &str, name: &str, text: &str) -> ID {
    let id = world
        .make(tag)
        .name(name)
        .prose(text)
        .var(Scenery)
        .build();

    put_in(world, id, loc);

    id
}

/// Adds a bit of backstory to be revealed when the conditions are right.
/// Backstory will appear only once.
fn make_story(world: &mut World, tag: &str, predicate: RulePred, story: &str) {
    let id = make_prose(world, &format!("{}-{}", tag, "Prose"), story);
    make_rule(world, &format!("{}-{}", tag, "Rule"), predicate, Action::PrintProse(id));
}

/// Adds a bit of prose to the scenario, for use by other entities.
fn make_prose(world: &mut World, tag: &str, prose: &str) -> ID {
    world
        .make(tag)
        .prose(prose)
        .build()
}

/// Adds a rule to the scenario, to be executed when the conditions are met.
/// The rule will execute only once.
fn make_rule(world: &mut World, tag: &str, predicate: RulePred, action: Action) {
    world
        .make(tag)
        .rule(predicate, action, true)
        .build();
}

/// Links one room to another in the given direction.
/// Links are not bidirectional.  If you want links both ways, you
/// have to add them.
fn oneway(world: &mut World, dir: Dir, from: ID, to: ID) {
    let room = &mut world.get(from).as_room();
    room.links.insert(dir, to);
    room.save(world);
}

/// Establishes a bidirectional link between two rooms.
fn connect(world: &mut World, from_dir: Dir, from: ID, to_dir: Dir, to: ID) {
    oneway(world, from_dir, from, to);
    oneway(world, to_dir, to, from);
}

/// Puts the thing in the container's inventory, and sets the thing's location.
/// No op if the thing is already in the location.
pub fn put_in(world: &mut World, thing: ID, container: ID) {
    if let Some(inv) = &mut world.entities[container].inventory {
        if !inv.contains(&thing) {
            inv.insert(thing);
            world.entities[thing].loc = Some(container);
        }
    }
}
