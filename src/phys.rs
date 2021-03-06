//! Physical System
//!
//! This system is responsible for moving entities from place to place, tracking
//! where they are, and making them available.  As such, it is concerned with the
//! location and inventory components.

use crate::entity::ID;
use crate::rule;
use crate::types::Dir;
use crate::types::Event::*;
use crate::types::LinkDest;
use crate::types::Flag::*;
use crate::visual;
use crate::world::World;
use crate::world::LIMBO;
use std::collections::BTreeSet;

type PhysResult = Result<(), String>;

//--------------------------------------------------------------------------------
// Queries

/// Returns an entity's location.
///
/// * Panics if the entity isn't the sort of thing that has a location.
pub fn loc(world: &World, thing: ID) -> ID {
    assert_has_location(world, thing);

    world.locations[&thing].id
}

/// Tries to follow a link in the given direction; returns the linked
/// location if any.
pub fn follow_link(world: &World, loc: ID, dir: Dir) -> Option<LinkDest> {
    assert_is_room(world, loc);

    let roomc = &world.rooms[&loc];

    roomc.links.get(&dir).cloned()
}

/// Determines whether the thing is in the container.
///
/// * Panics if the container has no inventory component.
/// * Panics if the thing has no location component.
pub fn owns(world: &World, container: ID, thing: ID) -> bool {
    assert_has_inventory(world, container);
    assert_has_location(world, thing);

    world.inventories[&container].has(thing)
}

/// Returns the contents of the container.  The result is a clone of the
/// container's inventory; the caller can mutate the container while
/// iterating over the list.
pub fn contents(world: &World, container: ID) -> BTreeSet<ID> {
    assert_has_inventory(world, container);

    world.inventories[&container].things.clone()
}

pub fn scenery(world: &World, owner: ID) -> BTreeSet<ID> {
    assert_has_inventory(world, owner);

    let mut result: BTreeSet<ID> = BTreeSet::new();

    // FIRST, get everything that's flagged as scenery.
    for id in contents(world, owner) {
        if world.has_flag(id, Scenery) {
            result.insert(id);
        }
    }

    result
}

pub fn non_scenery(world: &World, owner: ID) -> BTreeSet<ID> {
    assert_has_inventory(world, owner);

    let mut result: BTreeSet<ID> = BTreeSet::new();

    // FIRST, get everything that's flagged as scenery.
    for id in contents(world, owner) {
        if !world.has_flag(id, Scenery) {
            result.insert(id);
        }
    }

    result
}

pub fn immovable(world: &World, owner: ID) -> BTreeSet<ID> {
    assert_has_inventory(world, owner);

    let mut result: BTreeSet<ID> = BTreeSet::new();

    // FIRST, get everything that's flagged as immovable.
    for id in contents(world, owner) {
        if world.has_flag(id, Immovable) {
            result.insert(id);
        }
    }

    result
}


/// Finds all things in the viewer's location that are visible to
/// the viewer.  This includes things owned by the viewer, present
/// in the viewer's location, or (ultimately) visible in open containers.
pub fn visible(world: &World, viewer: ID) -> BTreeSet<ID> {
    let mut result: BTreeSet<ID> = BTreeSet::new();

    // FIRST, get anything owned by the viewer
    if world.has_inventory(viewer) {
        result.append(&mut contents(world, viewer));
    }

    // NEXT, get anything in the viewer's location.
    if world.has_location(viewer) {
        result.append(&mut contents(world, loc(world, viewer)));
    }

    result
}

/// Finds all things in the location's inventory that can be removed,
/// i.e., that isn't flagged as Immovable.
pub fn removable(world: &World, loc: ID) -> BTreeSet<ID> {
    assert_has_inventory(world, loc);

    let mut result: BTreeSet<ID> = BTreeSet::new();

    // FIRST, get everything owned by the viewer that isn't flagged
    // as Immovable.
    for id in contents(world, loc) {
        if !world.has_flag(id, Immovable){
            result.insert(id);
        }
    }

    result
}

/// Finds all things in the viewer's inventory that he could, in theory,
/// drop into his location
pub fn droppable(world: &World, viewer: ID) -> BTreeSet<ID> {
    assert_has_inventory(world, viewer);
    removable(world, viewer)
}

/// Finds all things in the viewer's location that he could, in theory,
/// move to his own inventory, i.e., all things that aren't flagged
/// as immovable.
pub fn gettable(world: &World, viewer: ID) -> BTreeSet<ID> {
    assert_has_location(world, viewer);

    let mut result: BTreeSet<ID> = BTreeSet::new();

    // FIRST, get everything in the current location that isn't
    // flagged as Immovable.
    for id in contents(world, loc(world, viewer)) {
        if !world.has_flag(id, Immovable) {
            result.insert(id);
        }
    }

    result
}

//--------------------------------------------------------------------------------
// Low-level operations
//
// These operations move things about and do the bookkeeping; but they contain no
// game logic.


/// Removes the thing from its current location and puts it in LIMBO.
pub fn take_out(world: &mut World, thing: ID) {
    let container = loc(world, thing);

    // FIRST, remove it from wherever
    world.inventories.get_mut(&container).unwrap().remove(thing);

    // NEXT, put it in LIMBO
    world.locations.get_mut(&thing).unwrap().id = LIMBO;
    world.inventories.get_mut(&LIMBO).unwrap().add(thing);
}

pub fn put_in(world: &mut World, thing: ID, container: ID) {
    // FIRST, remove it from wherever.
    let there = loc(world, thing);
    world.inventories.get_mut(&there).unwrap().remove(thing);

    // NEXT, put it where it goes.
    world.locations.get_mut(&thing).unwrap().id = container;
    world.inventories.get_mut(&container).unwrap().add(thing);
}

//---------------------------------------------------------------------------------
// High-level operations

/// The player tries to enter the room.
pub fn enter_room(world: &mut World, pid: ID, room: ID) -> PhysResult {
    if rule::allows(world, &EnterRoom(pid, room)) {
        put_in(world, pid, room);

        if !world.has_flag(pid, Seen(room)) {
            visual::room(world, room);
        } else {
            visual::room_brief(world, room);
        }

        world.set_flag(pid, Seen(room));

        rule::fire_event(world, &EnterRoom(pid, room));
    }

    Ok(())
}

/// The player gets the thing.
pub fn get_thing(world: &mut World, pid: ID, thing: ID) -> PhysResult {
    if rule::allows(world, &GetThing(pid, thing)) {
        put_in(world, thing, pid);
        visual::act("Taken.");
        rule::fire_event(world, &GetThing(pid, thing));
    }

    Ok(())
}

/// The player reads the thing's Book prose.
pub fn read_thing(world: &mut World, pid: ID, thing: ID) -> PhysResult {
    if rule::allows(world, &ReadThing(pid, thing)) {
        visual::read(world, thing);
        rule::fire_event(world, &ReadThing(pid, thing));
    }

    Ok(())
}

//--------------------------------------------------------------------------------
// Standard Assertions

fn idtag(world: &World, id: ID) -> String {
    format!("[{}] {}", id, world.tag(id))
}

fn assert_is_room(world: &World, loc: ID) {
    assert!(world.is_room(loc), "Not a room: {}", idtag(world, loc));
}

fn assert_has_inventory(world: &World, container: ID) {
    assert!(
        world.inventories.get(&container).is_some(),
        "Has no inventory component: {}",
        idtag(world, container)
    );
}

fn assert_has_location(world: &World, thing: ID) {
    assert!(
        world.locations.get(&thing).is_some(),
        "Has no location component: {}",
        idtag(world, thing)
    );
}
