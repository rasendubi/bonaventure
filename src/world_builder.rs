//! A scenario builder.
//!
//! It provides builder methods for each kind of entity that the framework supports, with
//! various aids.

use std::collections::HashSet;
use crate::entity::ID;
use crate::entity::flag_set_component::*;
use crate::entity::inventory_component::*;
use crate::entity::location_component::*;
use crate::entity::player_component::*;
use crate::entity::prose_component::*;
use crate::entity::room_component::*;
use crate::entity::rule_component::*;
use crate::entity::thing_component::*;
use crate::phys;
use crate::player_control::CommandHandler;
use crate::types::*;
use crate::world::World;

//-----------------------------------------------------------------------------------------------
// Constants

/// The tag of the LIMBO entity, where currently unused entities live.
pub const LIMBO: &str = "LIMBO";

/// The tag of the PLAYER entity.
pub const PLAYER: &str = "PLAYER";

/// Events for which rules can be written.
pub enum WBEvent<'a> {
    /// The player gets (or tries to get) the tagged entity
    GetThing(&'a str),

    /// The player reads (or tries to read) the tagged entity
    ReadThing(&'a str),

    /// The player enters (or tries to enter) the tagged entity
    EnterRoom(&'a str),
}

/// Expectations, to be checked when world-building is complete.
#[derive(Eq, PartialEq, Hash)]
enum Is {
    /// The entity has readable prose
    Book(ID),

    /// The entity is the player
    Player(ID),

    /// The entity is a room.
    Room(ID),

    /// The entity is a thing.
    Thing(ID),
}

//-----------------------------------------------------------------------------------------------
// The World Builder

/// # WorldBuilder
///
/// This struct is used to build game worlds.  It provides an API intended to make it
/// as easy and painless as possible to add game entities to the world with a minimum
/// of error.  As such, it's a precursor to a compiler for a game definition file
/// format.
///
/// The usage pattern is as follows:
///
/// * Create an instance of WorldBuilder.
/// * Use the player(), room(), thing(), etc., methods to add and configure game entities.
/// * When the scenario is complete, the world() method returns the newly created world.
///
/// Two entities are created automatically: LIMBO and PLAYER.  LIMBO is an entity
/// containing only an inventory set; it's a place to park things that shouldn't yet
/// or should no longer be visible in the game; and the PLAYER is, of course, the player.
pub struct WorldBuilder {
    world: World,
    expectations: HashSet<Is>,
}

impl WorldBuilder {
    //-------------------------------------------------------------------------------------------
    // Public Methods

    /// Creates a new world with LIMBO and a player with default settings.
    pub fn new() -> Self {
        // FIRST, create the new world
        let mut this = Self {
            world: World::new(),
            expectations: HashSet::new(),
        };

        // NEXT, create LIMBO, the container for things which aren't anywhere else.
        let limbo = this.world.alloc(LIMBO);
        assert!(limbo == 0);
        this.add_inventory(limbo);

        // NEXT, create the player basics.  The scenario can customize the player
        // as needed.
        let pid = this.world.alloc(PLAYER);
        this.world.pid = pid;

        this.world.players.insert(pid, PlayerComponent::new());
        this.world.things.insert(pid, ThingComponent::new("Yourself", "self"));
        this.add_inventory(pid);
        this.add_location(pid);
        this.add_flag(pid, Flag::Scenery);

        this
    }

    /// Adds a custom command consisting of a single verb.
    pub fn verb(&mut self, word: &str, hook: CommandHook) {
        // TODO: Add to list of verbs
        self.world.command_handlers.push(CommandHandler::verb(word, hook));
    }

    /// Adds a custom command triggered by a specific verb and noun.
    pub fn verb_noun(&mut self, verb: &str, noun: &str, hook: CommandHook) {
        // TODO: Add to list of verbs
        self.world.command_handlers.push(CommandHandler::verb_noun(verb, noun, hook));
    }

    /// Adds a custom command triggered by a specific verb and a noun representing
    /// a thing that's visible to the player.
    pub fn verb_visible(&mut self, verb: &str, hook: CommandHook) {
        // TODO: Add to list of verbs
        self.world.command_handlers.push(CommandHandler::verb_visible(verb, hook));
    }

    /// Configures the player.
    pub fn player(&mut self) -> PlayerBuilder {
        PlayerBuilder {
            wb: self,
        }
    }

    /// Creates or configures a room.
    pub fn room(&mut self, tag: &str, name: &str) -> RoomBuilder {
        let id = self.world.alloc(tag);

        self.world.rooms.insert(id, RoomComponent::new(name));
        self.add_inventory(id);
        self.add_flag_set(id);

        RoomBuilder {
            wb: self,
            tag: tag.to_string(),
            id,
        }
    }

    /// Creates or configures a feature, i.e., a thing that's a part of its container:
    /// the player's hands, a pool of water, a big machine.  Features are things that
    /// have their Scenery and Immovable flags set.
    pub fn feature(&mut self, tag: &str, name: &str, noun: &str) -> ThingBuilder {
        self.thing(tag, name, noun)
            .flag(Flag::Immovable)
            .flag(Flag::Scenery)
    }

    /// Creates or configures a thing.
    pub fn thing(&mut self, tag: &str, name: &str, noun: &str) -> ThingBuilder {
        let id = self.world.alloc(tag);

        self.world.things.insert(id, ThingComponent::new(name, noun));
        self.add_location(id);
        self.add_flag_set(id);

        ThingBuilder {
            wb: self,
            tag: tag.to_string(),
            id,
        }
    }

    /// Creates and configures a rule that will be triggered every turn.
    pub fn rule(&mut self, tag: &str) -> RuleBuilder {
        let id = self.world.alloc(tag);

        self.world.rules.insert(id, RuleComponent::new());
        self.add_flag_set(id);

        RuleBuilder {
            wb: self,
            tag: tag.to_string(),
            id,
        }
    }

    /// Creates and configures a guard that will determined whether a specific
    /// event can occur.  If the answer is no, then the guard can take some
    /// actions.
    pub fn allow(&mut self, evt: &WBEvent) -> RuleBuilder {
        let mut rulec = RuleComponent::new();
        rulec.is_guard = true;
        self.build_event_rule("allow", evt, rulec)
    }

    /// Creates and configures a rule that will be triggered when a specific
    /// event occurs.
    pub fn on(&mut self, evt: &WBEvent) -> RuleBuilder {
        let rulec = RuleComponent::new();
        self.build_event_rule("on", evt, rulec)
    }


    /// Completes world-building, after checking that all expectations are met.
    pub fn world(self) -> World {
        for expectation in self.expectations {
            match expectation {
                Is::Book(id) => {
                    assert!(self.world.has_prose_type(id, ProseType::Book),
                        "Expected book prose: [{}] {}",
                        id, self.world.tag(id));
                }
                Is::Player(id) => {
                    assert!(self.world.is_player(id),
                        "Expected player: [{}] {}",
                        id, self.world.tag(id));
                }
                Is::Room(id) => {
                    assert!(self.world.is_room(id),
                        "Expected room: [{}] {}",
                        id, self.world.tag(id));
                }
                Is::Thing(id) => {
                    assert!(self.world.is_thing(id),
                        "Expected thing: [{}] {}",
                        id, self.world.tag(id));
                }
            }
        }
        self.world
    }

    //-------------------------------------------------------------------------------------------
    // Utility methods

    /// Adds an expectation for later checking.
    fn expect(&mut self, expectation: Is) {
        self.expectations.insert(expectation);
    }

    /// Adds a location to an entity if it doesn't have one.  The entity will initially
    /// be in LIMBO.
    fn add_location(&mut self, id: ID) {
        if self.world.locations.get(&id).is_none() {
            self.world.locations.insert(id, LocationComponent::new());
        }
    }

    /// Sets the location of the thing to the entity with the given tag, creating
    /// the containing entity if need be.
    fn set_location(&mut self, thing: ID, loc_tag: &str) {
        // FIRST, make sure that the location exists and has an inventory.
        let loc = self.world.alloc(loc_tag);
        self.add_inventory(loc);

        // NEXT, make sure that the thing has a location.
        self.add_location(thing);

        // NEXT, put the thing in the location.
        phys::put_in(&mut self.world, thing, loc);
    }

    /// Adds an inventory to an entity if it doesn't have one.
    fn add_inventory(&mut self, id: ID) {
        if self.world.inventories.get(&id).is_none() {
            self.world.inventories.insert(id, InventoryComponent::new());
        }
    }

    /// Adds a flag set to an entity if it doesn't have one.
    fn add_flag_set(&mut self, id: ID) {
        if self.world.flag_sets.get(&id).is_none() {
            self.world.flag_sets.insert(id, FlagSetComponent::new());
        }
    }

    /// Adds a specific flag to the entity, creating the flag set component if
    /// necessary.
    fn add_flag(&mut self, id: ID, flag: Flag) {
        self.add_flag_set(id);
        self.world.set_flag(id, flag);
    }

    /// Adds a prose component to an entity if it doesn't have one.
    fn add_prose_component(&mut self, id: ID) {
        if self.world.proses.get(&id).is_none() {
            self.world.proses.insert(id, ProseComponent::new());
        }
    }

    /// Adds a prose string of a given type to an entity's prose component,
    /// creating the component if necessary.
    fn add_prose(&mut self, id: ID, prose_type: ProseType, text: &str) {
        self.add_prose_component(id);

        let prose = Prose::Prose(text.trim().into());
        self.world.proses.get_mut(&id).unwrap().types.insert(prose_type, prose);
    }

    /// Adds a prose hook of a given type to an entity's prose component,
    /// creating the component if necessary.
    fn add_prose_hook(&mut self, id: ID, prose_type: ProseType, hook: EntityProseHook) {
        self.add_prose_component(id);

        let prose = Prose::Hook(ProseHook::new(hook));
        self.world.proses.get_mut(&id).unwrap().types.insert(prose_type, prose);
    }

    /// Creates and configures a rule that will be triggered when a specific
    /// event occurs.
    fn build_event_rule(&mut self, kind: &str, evt: &WBEvent, mut rulec: RuleComponent) -> RuleBuilder {
        // FIRST, compute the internal event.
        let tag: String = match evt {
            WBEvent::GetThing(thing_tag) => {
                let tid = self.world.alloc(thing_tag);
                rulec.event = Event::GetThing(self.world.pid, tid);
                self.expect(Is::Thing(tid));
                format!("{}-get-{}", kind, thing_tag)
            }
            WBEvent::ReadThing(thing_tag) => {
                let tid = self.world.alloc(thing_tag);
                rulec.event = Event::ReadThing(self.world.pid, tid);
                self.expect(Is::Thing(tid));
                self.expect(Is::Book(tid));
                format!("{}-read-{}", kind, thing_tag)
            }
            WBEvent::EnterRoom(room_tag) => {
                let rid = self.world.alloc(room_tag);
                rulec.event = Event::EnterRoom(self.world.pid, rid);
                self.expect(Is::Room(rid));
                format!("{}-enter-{}", kind, room_tag)
            }
        };

        let id = self.world.alloc(&tag);
        self.world.rules.insert(id, rulec);
        self.add_flag_set(id);

        RuleBuilder {
            wb: self,
            tag: tag,
            id,
        }
    }
}

/// # PlayerBuilder -- A tool for configuring the player entity.
///
/// WorldBuilder creates and initializes the player automatically; this struct allows
/// the scenario author to configure scenario-specific features.
pub struct PlayerBuilder<'a> {
    wb: &'a mut WorldBuilder,
}

impl<'a> PlayerBuilder<'a> {
    /// Sets the player's initial location given the location's tag
    pub fn location(self, loc_tag: &str) -> PlayerBuilder<'a> {
        self.wb.set_location(self.wb.world.pid, loc_tag);
        let loc = self.wb.world.lookup(loc_tag);
        self.wb.add_flag(self.wb.world.pid, Flag::Seen(loc));
        self.wb.expect(Is::Room(loc));

        self
    }

    /// Adds descriptive prose to the player.
    pub fn on_examine(self, text: &str) -> PlayerBuilder<'a> {
        self.wb.add_prose(self.wb.world.pid, ProseType::Thing, text);
        self
    }

    /// Adds a prose hook to the player, to produce descriptive prose
    /// on demand.
    pub fn on_examine_hook(self, hook: EntityProseHook) -> PlayerBuilder<'a> {
        self.wb.add_prose_hook(self.wb.world.pid, ProseType::Thing, hook);
        self
    }

    pub fn flag(self, flag: Flag) -> PlayerBuilder<'a> {
        self.wb.add_flag(self.wb.world.pid, flag);
        self
    }
}

/// # RoomBuilder -- A tool for creating and configuring room entities.
pub struct RoomBuilder<'a> {
    wb: &'a mut WorldBuilder,
    tag: String,
    id: ID,
}

impl<'a> RoomBuilder<'a> {
    /// Adds descriptive prose to the room.
    pub fn prose(self, text: &str) -> RoomBuilder<'a> {
        self.wb.add_prose(self.id, ProseType::Room, text);
        self
    }

    /// Adds a prose hook to the room, to produce descriptive prose
    /// on demand.
    pub fn prose_hook(self, hook: EntityProseHook) -> RoomBuilder<'a> {
        self.wb.add_prose_hook(self.id, ProseType::Room, hook);
        self
    }

    /// Sets a flag on the room.
    pub fn flag(self, flag: Flag) -> RoomBuilder<'a> {
        self.wb.add_flag(self.id, flag);
        self
    }

    /// Creates a link from this room to another room given the direction and
    /// the other room's tag.
    pub fn link(self, dir: Dir, room_tag: &str) -> RoomBuilder<'a> {
        // FIRST, get the id of the destination.
        let dest = self.wb.world.alloc(room_tag);
        self.wb.expect(Is::Room(dest));

        let link = LinkDest::Room(dest);
        self.wb.world.rooms.get_mut(&self.id).unwrap().links.insert(dir, link);

        self
    }

    /// Adds a dead end in the given direction.
    pub fn dead_end(self, dir: Dir, text: &str) -> RoomBuilder<'a> {
        let dead_end = LinkDest::DeadEnd(text.into());
        self.wb.world.rooms.get_mut(&self.id).unwrap().links.insert(dir, dead_end);
        self
    }
}

/// # ThingBuilder -- A tool for creating and configuring thing entities.
pub struct ThingBuilder<'a> {
    wb: &'a mut WorldBuilder,
    tag: String,
    id: ID,
}

impl<'a> ThingBuilder<'a> {
    /// Sets the thing's initial location given the location's tag.
    pub fn location(self, loc: &str) -> ThingBuilder<'a> {
        self.wb.set_location(self.id, loc);
        self
    }
    /// Adds descriptive prose to the thing.
    pub fn on_examine(self, text: &str) -> ThingBuilder<'a> {
        self.wb.add_prose(self.id, ProseType::Thing, text);
        self
    }

    /// Adds a prose hook to the thing, to produce descriptive prose
    /// on demand.
    pub fn on_examine_hook(self, hook: EntityProseHook) -> ThingBuilder<'a> {
        self.wb.add_prose_hook(self.id, ProseType::Thing, hook);
        self
    }

    /// Adds readable prose to the thing.
    pub fn on_read(self, text: &str) -> ThingBuilder<'a> {
        self.wb.add_prose(self.id, ProseType::Book, text);
        self
    }

    /// Adds a prose hook to the thing, to produce readable prose
    /// on demand.
    pub fn on_read_hook(self, hook: EntityProseHook) -> ThingBuilder<'a> {
        self.wb.add_prose_hook(self.id, ProseType::Book, hook);
        self
    }

    /// Adds scenery prose to the thing.
    pub fn on_scenery(self, text: &str) -> ThingBuilder<'a> {
        self.wb.add_prose(self.id, ProseType::Scenery, text);
        self
    }

    /// Adds a prose hook to the thing, to produce readable prose
    /// on demand.
    pub fn on_scenery_hook(self, hook: EntityProseHook) -> ThingBuilder<'a> {
        self.wb.add_prose_hook(self.id, ProseType::Scenery, hook);
        self
    }

    /// Sets a flag on the thing.
    pub fn flag(self, flag: Flag) -> ThingBuilder<'a> {
        self.wb.add_flag(self.id, flag);
        self
    }
}

/// # RuleBuilder -- A tool for creating and configuring rules.
pub struct RuleBuilder<'a> {
    wb: &'a mut WorldBuilder,
    tag: String,
    id: ID,
}

impl<'a> RuleBuilder<'a> {
    /// Specifies the predicate for normal rules.  If omitted, the rule fires every time it
    /// is triggered.
    pub fn when(self, predicate: RulePredicate) -> RuleBuilder<'a> {
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        assert!(!rulec.is_guard, "Cannot set 'when' predicate on allow() rule: {}", self.tag);
        rulec.predicate = predicate;
        self
    }

    /// Specifies the predicate for guard rules.  If omitted, the guard fires every time it
    /// is triggered.
    pub fn unless(self, predicate: RulePredicate) -> RuleBuilder<'a> {
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        assert!(rulec.is_guard, "Cannot set 'unless' predicate on normal rule: {}", self.tag);
        rulec.predicate = predicate;
        self
    }

    /// Specifies that the rule should execute no more than once.
    pub fn once_only(self) -> RuleBuilder<'a> {
        let rulec = &self.wb.world.rules[&self.id];
        assert!(!rulec.is_guard, "Cannot set 'once_only' predicate on allow() rule: {}", self.tag);
        self.wb.add_flag(self.id, Flag::FireOnce);
        self
    }

    /// Specifies text to print when the rule fires.
    pub fn print(self, text: &str) -> RuleBuilder<'a> {
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        rulec.script.print(text);

        self
    }

    /// Sets a flag on the entity.
    pub fn set_flag(self, tag: &str, flag: Flag) -> RuleBuilder<'a> {
        // FIRST, get the entity on which we'll be adding the flag, and
        // make sure it's the kind of thing we can set a flag on.
        let id = self.wb.world.alloc(tag);
        self.wb.add_flag_set(id);

        // NEXT, add the action.
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        rulec.script.set_flag(tag, flag);
        self
    }

    /// Unsets a flag on the entity.
    pub fn unset_flag(self, tag: &str, flag: Flag) -> RuleBuilder<'a> {
        // FIRST, get the entity on which we'll be adding the flag, and
        // make sure it's the kind of thing we can set a flag on.
        let id = self.wb.world.alloc(tag);
        self.wb.add_flag_set(id);

        // NEXT, add the action.
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        rulec.script.unset_flag(tag, flag);
        self
    }

    /// Moves a thing to LIMBO
    pub fn forget(self, thing: &str) -> RuleBuilder<'a> {
        // FIRST, get the entity which we'll be forgetting.
        let id = self.wb.world.alloc(thing);
        self.wb.expect(Is::Thing(id));

        // NEXT, add the action.
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        rulec.script.forget(thing);
        self
    }

    /// Kills the tagged entity, i.e., sets the Dead flag.
    /// TODO: At present, really presumes that the entity is the player.
    /// Eventually, we might have NPCs, monsters, etc.  But the script
    /// action would need to be updated as well, in that case.
    pub fn kill(self, tag: &str) -> RuleBuilder<'a> {
        let id = self.wb.world.lookup(tag);
        self.wb.expect(Is::Player(id));
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        rulec.script.kill(tag);
        self
    }

    /// Revives the tagged entity, i.e., clears the Dead flag.
    /// TODO: At present, really presumes that the entity is the player.
    /// Eventually, we might have NPCs, monsters, etc.  But the script
    /// action would need to be updated as well, in that case.
    pub fn revive(self, tag: &str) -> RuleBuilder<'a> {
        let id = self.wb.world.lookup(tag);
        self.wb.expect(Is::Player(id));
        let rulec = &mut self.wb.world.rules.get_mut(&self.id).unwrap();
        rulec.script.revive(tag);
        self
    }
}
