//! Data structures supporting tables of interned objects.
//! These tables form the 'sections' of our FLOs.

use std::{collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};

use crate::{poison::Poisonable, types::PoisonType};

/// Type for all integer-based identifiers used in interning logic.
pub(crate) type InternIdentifier = usize;

/// The special table value that always contains a poisoned element.
const POISON_ENTRY: usize = 0xdecea5ed;

/// Interning Table type -- a type made for generic tables of
/// interned objects; used to store these objects in the
#[derive(Serialize, Deserialize, Debug)]
pub struct InternTable<IdType, ValueType>
where
    IdType: Eq + Hash + From<usize> + Into<usize> + Copy,
    ValueType: Poisonable + Clone,
{
    // The internal bidirectional hash-map used for interning.
    table: HashMap<usize, ValueType>,

    // The next ID to be assigned.
    next_id: usize,

    // Mark our IdType as used.
    _phantom: std::marker::PhantomData<IdType>,
}

impl<IdType, ValueType> InternTable<IdType, ValueType>
where
    IdType: Eq + Hash + From<usize> + Into<usize> + Copy,
    ValueType: Poisonable + Clone,
{
    /// Creates a new intern table.
    pub fn new() -> InternTable<IdType, ValueType> {
        let mut s = InternTable {
            // Start our IDs at one, as we've reserved 0.
            table:   HashMap::new(),
            next_id: 1,

            _phantom: std::marker::PhantomData,
        };

        // Allocate our two special IDs.
        s.table.insert(
            0,
            ValueType::get_poison_value(PoisonType::NullInternedValue),
        );
        s.table.insert(
            POISON_ENTRY,
            ValueType::get_poison_value(PoisonType::Undefined),
        );

        s
    }

    /// Allocates an ID for internal use.
    fn allocate_id(&mut self) -> usize {
        // Get the next ID in our table.
        let allocated_id = self.next_id;
        let mut next_id = allocated_id + 1;

        // If this ID happens to be taken, move to the next one
        // until we find a free ID.
        while self.table.contains_key(&next_id) {
            next_id += 1;
        }

        // Store our new successor ID, and return the allocated one.
        self.next_id = next_id;
        allocated_id
    }

    /// Inserts a new value into the intern table, getting its ID.
    pub fn insert(&mut self, v: &ValueType) -> IdType {
        // Place the object into a new slot...
        let id = self.allocate_id();

        self.table.insert(id, v.clone());

        // ... and return the allocated ID.
        id.into()
    }

    /// Retrieves a value from the intern table by ID.
    /// Panics if the ID does not exist.
    pub fn get(&self, id: IdType) -> ValueType {
        let raw_id: usize = id.into();
        self.table
            .get(&raw_id)
            .expect("internal consistency error: get with an unknown ID!")
            .clone()
    }

    /// Places a value into the intern table at a given ID, which _must_
    /// have already been allocated by a previous call to insert().
    ///
    /// Panics if the relevant ID is not present.
    pub fn swap(&mut self, id: IdType, value: ValueType) -> ValueType {
        let raw_id: usize = id.into();

        // Update both sides of the table.
        let previous = self.table.insert(raw_id, value.clone());

        previous.expect("internal consistency error: swap called on an unknown ID!")
    }
}
