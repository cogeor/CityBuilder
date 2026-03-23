//! Commands — player actions queued and applied to world state.

use city_core::{ArchetypeId, TileCoord, EntityHandle};
use serde::{Deserialize, Serialize};

/// A command issued by the player, queued for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Place a building at a tile position.
    PlaceBuilding {
        archetype_id: ArchetypeId,
        position: TileCoord,
        rotation: u8,
    },
    /// Demolish an existing entity.
    Demolish {
        handle: EntityHandle,
    },
    /// Toggle enable/disable on an entity.
    ToggleEnabled {
        handle: EntityHandle,
    },
    /// Set a zone on a tile.
    SetZone {
        position: TileCoord,
        zone_id: u8,
    },
    /// Build a road segment.
    BuildRoad {
        from: TileCoord,
        to: TileCoord,
    },
}

/// Queue of pending commands to be applied during PreTick.
#[derive(Debug, Default)]
pub struct CommandQueue {
    queue: Vec<Command>,
}

impl CommandQueue {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, cmd: Command) {
        self.queue.push(cmd);
    }

    pub fn drain(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.queue)
    }

    pub fn len(&self) -> usize { self.queue.len() }
    pub fn is_empty(&self) -> bool { self.queue.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_drain() {
        let mut q = CommandQueue::new();
        q.push(Command::PlaceBuilding {
            archetype_id: 1,
            position: TileCoord::new(5, 10),
            rotation: 0,
        });
        q.push(Command::Demolish {
            handle: EntityHandle::new(0, 0),
        });
        assert_eq!(q.len(), 2);
        let cmds = q.drain();
        assert_eq!(cmds.len(), 2);
        assert!(q.is_empty());
    }
}
