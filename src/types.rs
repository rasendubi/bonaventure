//! Type definitions for this app.

use crate::world::*;
use std::collections::hash_map::HashMap;
use std::collections::hash_set::HashSet;

/// The entity ID type: an integer.
pub type ID = usize;

/// Directions
#[derive(PartialEq, Eq, Hash, Debug)]
#[allow(dead_code)]
pub enum Dir {
    North,
    South,
    East,
    West,
    Up,
    Down,
    In,
    Out,
}

/// Entity prose
pub struct ProseComponent {
    pub text: String,
}

/// Inter-room links
pub struct LinksComponent {
    pub map: HashMap<Dir, ID>,
}

impl LinksComponent {
    pub fn new() -> LinksComponent {
        LinksComponent {
            map: HashMap::new(),
        }
    }
}

/// A Thing is something that can be in a location and that the user can
/// interact with.  This structure contains details about Things, i.e.,
/// are they portable?
#[derive(Debug)]
pub struct ThingComponent {
    pub portable: bool,
}

/// An Inventory is a list of things contained with the current entity.
#[derive(Debug)]
pub struct InventoryComponent {
    pub things: HashSet<ID>,
}

impl InventoryComponent {
    pub fn new() -> InventoryComponent {
        InventoryComponent {
            things: HashSet::new(),
        }
    }
}

/// Actions taken by rules (and maybe other things)
#[derive(Debug)]
pub enum Action {
    Print,
}

/// Game rules: actions taken when a predicate is met, and probably never repeated.
pub struct RuleComponent {
    pub predicate: Box<Fn(&World) -> bool>,
    pub action: Action,
    pub once_only: bool,
    pub fired: bool,
}

impl RuleComponent {
    pub fn new<F: 'static>(predicate: F, action: Action, once_only: bool) -> RuleComponent
    where
        F: Fn(&World) -> bool,
    {
        RuleComponent {
            predicate: Box::new(predicate),
            action,
            once_only,
            fired: false,
        }
    }
}

/// Player Component: Special data about the player
pub struct PlayerComponent {
    pub seen: HashSet<ID>,
}

impl PlayerComponent {
    pub fn new() -> PlayerComponent {
        PlayerComponent {
            seen: HashSet::new(),
        }
    }
}
