// ------------------------------------------------------------
// Module: queue
// Purpose: Playback order, shuffle, and repeat. The visible queue
//          stays in the order tracks were added. Shuffle only
//          changes which track Next and Previous choose.
// ------------------------------------------------------------

use std::time::Duration;

use rand::RngExt;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::library::Track;

/// Previous restarts the current track after this much playback.
const RESTART_AFTER: Duration = Duration::from_secs(3);

// ------------------------------------------------------------
// Enum: RepeatMode
// Purpose: What happens when the current track finishes.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

// ------------------------------------------------------------
// Enum: Step
// Purpose: Result of moving through the queue.
//   Play     - cursor changed, start that track
//   Restart  - same track, from the beginning
//   End      - nothing left to play
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Play,
    Restart,
    End,
}

// ------------------------------------------------------------
// Struct: Queue
// Purpose: Owns the tracks and the playback cursor.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Queue {
    tracks: Vec<Track>,
    /// Indexes into `tracks`, in playback order.
    order: Vec<usize>,
    cursor: usize,
    shuffle: bool,
    repeat: RepeatMode,
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    // --------------------------------------------------------
    // Function: new
    // Purpose: Empty queue, shuffle off, repeat off.
    // --------------------------------------------------------
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            order: Vec::new(),
            cursor: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
        }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn shuffle_enabled(&self) -> bool {
        self.shuffle
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat
    }

    // --------------------------------------------------------
    // Function: persistable_paths
    // Purpose: Local files to write into the session queue.
    //          The index follows those files, not skipped streams.
    // --------------------------------------------------------
    pub fn persistable_paths(&self) -> (Vec<std::path::PathBuf>, Option<usize>) {
        let current = self.current_display_index();
        let mut paths = Vec::new();
        let mut saved_index = None;
        for (index, track) in self.tracks.iter().enumerate() {
            if !track.is_persistable() {
                continue;
            }
            if current == Some(index) {
                saved_index = Some(paths.len());
            }
            paths.push(track.path.clone());
        }
        (paths, saved_index)
    }

    pub fn current_mut(&mut self) -> Option<&mut Track> {
        let index = self.current_display_index()?;
        self.tracks.get_mut(index)
    }

    // --------------------------------------------------------
    // Function: current
    // Purpose: The track that Play / Next / Previous act on.
    // --------------------------------------------------------
    pub fn current(&self) -> Option<&Track> {
        self.current_display_index()
            .and_then(|index| self.tracks.get(index))
    }

    // --------------------------------------------------------
    // Function: current_display_index
    // Purpose: Index in the visible queue, not the shuffle order.
    // --------------------------------------------------------
    pub fn current_display_index(&self) -> Option<usize> {
        self.order.get(self.cursor).copied()
    }

    // --------------------------------------------------------
    // Function: push_many
    // Purpose: Appends tracks. Shuffle inserts them after the
    //          current track in random playback slots.
    // --------------------------------------------------------
    pub fn push_many(&mut self, tracks: Vec<Track>) {
        for track in tracks {
            let index = self.tracks.len();
            self.tracks.push(track);
            self.insert_playback_index(index);
        }
    }

    // --------------------------------------------------------
    // Function: select_display
    // Purpose: Moves the cursor to a visible row.
    // --------------------------------------------------------
    pub fn select_display(&mut self, display_index: usize) -> bool {
        match self.order.iter().position(|index| *index == display_index) {
            Some(position) => {
                self.cursor = position;
                true
            }
            None => false,
        }
    }

    // --------------------------------------------------------
    // Function: remove_display
    // Purpose: Removes a visible row and repairs the cursor.
    // Output: True when the removed row was the current track.
    // --------------------------------------------------------
    pub fn remove_display(&mut self, display_index: usize) -> bool {
        if display_index >= self.tracks.len() {
            return false;
        }
        let removed_current = self.current_display_index() == Some(display_index);
        let order_pos = self.order.iter().position(|index| *index == display_index);
        self.tracks.remove(display_index);
        self.order.retain(|index| *index != display_index);
        for index in &mut self.order {
            if *index > display_index {
                *index -= 1;
            }
        }
        if removed_current {
            if self.order.is_empty() {
                self.cursor = 0;
            } else if let Some(position) = order_pos {
                self.cursor = position.min(self.order.len() - 1);
            }
        } else if let Some(position) = order_pos {
            if position < self.cursor {
                self.cursor -= 1;
            }
        }
        if !self.order.is_empty() && self.cursor >= self.order.len() {
            self.cursor = self.order.len() - 1;
        }
        removed_current
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.order.clear();
        self.cursor = 0;
    }

    // --------------------------------------------------------
    // Function: play_next
    // Purpose: Moves a visible row so it is the next track in
    //          playback order. Shuffle only affects `order`, so
    //          the chosen track still plays next. The visible
    //          list is left as-is.
    // Output: False when the row does not exist.
    // --------------------------------------------------------
    pub fn play_next(&mut self, display_index: usize) -> bool {
        if display_index >= self.tracks.len() {
            return false;
        }
        let Some(order_pos) = self.order.iter().position(|index| *index == display_index) else {
            return false;
        };
        if order_pos == self.cursor {
            return true;
        }
        let index = self.order.remove(order_pos);
        if order_pos < self.cursor {
            self.cursor -= 1;
        }
        let insert_at = (self.cursor + 1).min(self.order.len());
        self.order.insert(insert_at, index);
        true
    }

    // --------------------------------------------------------
    // Function: insert_next
    // Purpose: Adds a track at the end of the visible queue and
    //          slots it immediately after the cursor. An empty
    //          queue starts on this track.
    // --------------------------------------------------------
    pub fn insert_next(&mut self, track: Track) {
        let index = self.tracks.len();
        self.tracks.push(track);
        if self.order.is_empty() {
            self.order.push(index);
            self.cursor = 0;
            return;
        }
        let insert_at = (self.cursor + 1).min(self.order.len());
        self.order.insert(insert_at, index);
    }

    // --------------------------------------------------------
    // Function: retarget
    // Purpose: Points every row that used `old` at a relocated file.
    // Output: How many rows changed.
    // --------------------------------------------------------
    pub fn retarget(&mut self, old: &std::path::Path, replacement: &Track) -> usize {
        let mut count = 0;
        for track in &mut self.tracks {
            if track.path == old {
                *track = replacement.clone();
                count += 1;
            }
        }
        count
    }

    // --------------------------------------------------------
    // Function: set_shuffle
    // Purpose: Turns shuffle on or off and rebuilds playback
    //          order without losing the current track.
    // --------------------------------------------------------
    pub fn set_shuffle(&mut self, enabled: bool) {
        if self.shuffle == enabled {
            return;
        }
        self.shuffle = enabled;
        self.rebuild_order();
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    // --------------------------------------------------------
    // Function: cycle_repeat
    // Purpose: Off -> All -> One -> Off.
    // --------------------------------------------------------
    pub fn cycle_repeat(&mut self) -> RepeatMode {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        self.repeat
    }

    // --------------------------------------------------------
    // Function: go_next
    // Purpose: Advances playback. Repeat One only restarts when
    //          the track ended by itself (`automatic`).
    // --------------------------------------------------------
    pub fn go_next(&mut self, automatic: bool) -> Step {
        if self.order.is_empty() {
            return Step::End;
        }
        if automatic && self.repeat == RepeatMode::One {
            return Step::Restart;
        }
        if self.cursor + 1 < self.order.len() {
            self.cursor += 1;
            Step::Play
        } else if self.repeat == RepeatMode::All {
            self.cursor = 0;
            Step::Play
        } else {
            Step::End
        }
    }

    // --------------------------------------------------------
    // Function: go_previous
    // Purpose: Restarts the current track after 3 seconds,
    //          otherwise moves to the previous track.
    // --------------------------------------------------------
    pub fn go_previous(&mut self, position: Duration) -> Step {
        if self.order.is_empty() {
            return Step::End;
        }
        if position > RESTART_AFTER {
            return Step::Restart;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
            return Step::Play;
        }
        if self.repeat == RepeatMode::All {
            self.cursor = self.order.len() - 1;
            return Step::Play;
        }
        Step::Restart
    }

    fn insert_playback_index(&mut self, track_index: usize) {
        if !self.shuffle || self.order.is_empty() {
            self.order.push(track_index);
            return;
        }
        let mut rng = rand::rng();
        let start = (self.cursor + 1).min(self.order.len());
        let slot = rng.random_range(start..self.order.len() + 1);
        self.order.insert(slot, track_index);
    }

    fn rebuild_order(&mut self) {
        let current = self.current_display_index();
        let mut indices: Vec<usize> = (0..self.tracks.len()).collect();
        if self.shuffle {
            if let Some(current) = current {
                indices.retain(|index| *index != current);
                indices.shuffle(&mut rand::rng());
                indices.insert(0, current);
            } else {
                indices.shuffle(&mut rand::rng());
            }
            self.cursor = 0;
        } else {
            self.cursor = current
                .and_then(|current| indices.iter().position(|index| *index == current))
                .unwrap_or(0);
        }
        self.order = indices;
    }
}

#[cfg(test)]
#[path = "queue_tests.rs"]
mod tests;
