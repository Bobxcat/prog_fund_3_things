use std::mem;

pub struct KeyMap<V> {
    slots: Vec<Slot<V>>,
    empty_head: u32,
}

impl<V> KeyMap<V> {
    pub fn new() -> Self {
        Self {
            slots: vec![],
            empty_head: u32::MAX,
        }
    }

    pub fn insert(&mut self, value: V) -> Key {
        self.insert_with_key(|_| value)
    }

    pub fn insert_with_key(&mut self, insert: impl FnOnce(Key) -> V) -> Key {
        match self.empty_head {
            u32::MAX => {
                let key = Key {
                    generation: 0,
                    idx: self.slots.len() as u32,
                };
                self.slots.push(Slot {
                    generation: 0,
                    kind: SlotKind::Occupied(insert(key)),
                });
                key
            }
            empty_head => {
                let slot = &mut self.slots[empty_head as usize];

                let SlotKind::Empty { next_empty } = slot.kind else {
                    unreachable!()
                };

                slot.generation += 1;
                self.empty_head = next_empty;

                Key {
                    generation: slot.generation,
                    idx: empty_head,
                }
            }
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<V> {
        let slot = &mut self.slots[key.idx as usize];

        if key.generation != slot.generation {
            return None;
        }
        if !slot.is_occupied() {
            return None;
        }

        self.empty_head = key.idx;

        slot.generation += 1;
        let SlotKind::Occupied(old_value) = mem::replace(
            &mut slot.kind,
            SlotKind::Empty {
                next_empty: self.empty_head,
            },
        ) else {
            unreachable!()
        };

        Some(old_value)
    }

    pub fn get(&self, key: Key) -> Option<&V> {
        let slot = self.slots.get(key.idx as usize)?;
        if key.generation != slot.generation {
            return None;
        }

        match &slot.kind {
            SlotKind::Occupied(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut V> {
        let slot = self.slots.get_mut(key.idx as usize)?;
        if key.generation != slot.generation {
            return None;
        }

        match &mut slot.kind {
            SlotKind::Occupied(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    generation: u32,
    idx: u32,
}

struct Slot<V> {
    generation: u32,
    kind: SlotKind<V>,
}

impl<V> Slot<V> {
    pub fn is_occupied(&self) -> bool {
        matches!(self.kind, SlotKind::Occupied(_))
    }
}

enum SlotKind<V> {
    Empty { next_empty: u32 },
    Occupied(V),
}

mod tests {
    enum Action {
        //
    }

    fn random_actions() {
        //
    }
}
