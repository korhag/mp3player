// ------------------------------------------------------------
// Module: groups
// Purpose: Favorite shows as a flat list of loose shows and
//          named groups. The show records stay in favorites.json;
//          this file only stores the order and the folders.
// ------------------------------------------------------------

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config;
use crate::podcast::Show;

/// Movement past this distance, in logical pixels, starts a drag.
pub const DRAG_THRESHOLD: f32 = 4.0;

// ------------------------------------------------------------
// Enum: FavoriteItem
// Purpose: One row in the favorites pane. A group holds feed URLs
//          in display order.
// ------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FavoriteItem {
    Show(String),
    Group {
        name: String,
        #[serde(default)]
        collapsed: bool,
        shows: Vec<String>,
    },
}

// ------------------------------------------------------------
// Enum: DropSlot
// Purpose: An insertion point. Indices refer to the list before
//          the dragged row is removed.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropSlot {
    /// Insert before this top-level row. `len` appends.
    Before(usize),
    /// Insert inside a group before this show. `shows.len()` appends.
    InGroup { group: usize, before: usize },
}

// ------------------------------------------------------------
// Enum: DropTarget
// Purpose: What the pointer is hovering while a drag is armed.
// ------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropTarget {
    OnShow(String),
    OnGroup(usize),
    Slot(DropSlot),
}

// ------------------------------------------------------------
// Enum: DropBand
// Purpose: Which part of a favorite row the pointer is on.
//          The edges reorder. The middle groups or joins.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropBand {
    Before,
    On,
    After,
}

// ------------------------------------------------------------
// Function: drop_zone
// Purpose: The edges reorder. When `halves` is set, the row is
//          split in two so a show already in this group can be
//          placed before or after the row under the pointer.
// ------------------------------------------------------------
pub fn drop_zone(local_y: f32, height: f32, halves: bool) -> DropBand {
    if height <= 0.0 {
        return DropBand::On;
    }
    if halves {
        if local_y < height / 2.0 {
            DropBand::Before
        } else {
            DropBand::After
        }
    } else {
        let edge = (height * 0.33).min(height / 3.0);
        if local_y <= edge {
            DropBand::Before
        } else if local_y >= height - edge {
            DropBand::After
        } else {
            DropBand::On
        }
    }
}

// ------------------------------------------------------------
// Function: drop_is_noop
// Purpose: True when moving this show to the slot would leave
//          it where it already is. That release is a click.
// ------------------------------------------------------------
pub fn drop_is_noop(items: &[FavoriteItem], feed: &str, slot: DropSlot) -> bool {
    locate(items, feed).is_some_and(|from| same_place(&from, slot))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragItem {
    Show(String),
    Group(usize),
}

// ------------------------------------------------------------
// Struct: DragGesture
// Purpose: A press that becomes a drag after the pointer moves.
// ------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub struct DragGesture {
    pub item: DragItem,
    pub origin_x: f32,
    pub origin_y: f32,
    pub armed: bool,
}

impl DragGesture {
    pub fn press(item: DragItem, x: f32, y: f32) -> Self {
        Self {
            item,
            origin_x: x,
            origin_y: y,
            armed: false,
        }
    }

    // --------------------------------------------------------
    // Method: follow
    // Purpose: Arms the drag once the pointer leaves the slop.
    // --------------------------------------------------------
    pub fn follow(&mut self, x: f32, y: f32) {
        if self.armed {
            return;
        }
        let dx = x - self.origin_x;
        let dy = y - self.origin_y;
        if dx * dx + dy * dy >= DRAG_THRESHOLD * DRAG_THRESHOLD {
            self.armed = true;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupChange {
    Created(usize),
    Joined(usize),
}

enum Located {
    Loose(usize),
    Grouped { group: usize, index: usize },
}

pub fn load_groups() -> Vec<FavoriteItem> {
    load_from(&config::favorite_groups_path())
}

pub fn save_groups(items: &[FavoriteItem]) -> std::io::Result<()> {
    save_to(&config::favorite_groups_path(), items)
}

fn load_from(path: &Path) -> Vec<FavoriteItem> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_to(path: &Path, items: &[FavoriteItem]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(items).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

// ------------------------------------------------------------
// Function: reconcile
// Purpose: Drops feeds that are no longer favorites and appends
//          new favorites as loose shows, in favorite order.
// ------------------------------------------------------------
pub fn reconcile(items: Vec<FavoriteItem>, favorites: &[Show]) -> Vec<FavoriteItem> {
    let known: HashSet<&str> = favorites
        .iter()
        .map(|show| show.feed_url.as_str())
        .collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        match item {
            FavoriteItem::Show(feed_url) => {
                if known.contains(feed_url.as_str()) && seen.insert(feed_url.clone()) {
                    out.push(FavoriteItem::Show(feed_url));
                }
            }
            FavoriteItem::Group {
                name,
                collapsed,
                shows,
            } => {
                let shows = shows
                    .into_iter()
                    .filter(|feed_url| {
                        known.contains(feed_url.as_str()) && seen.insert(feed_url.clone())
                    })
                    .collect::<Vec<_>>();
                if !shows.is_empty() {
                    out.push(FavoriteItem::Group {
                        name,
                        collapsed,
                        shows,
                    });
                }
            }
        }
    }
    for show in favorites {
        if seen.insert(show.feed_url.clone()) {
            out.push(FavoriteItem::Show(show.feed_url.clone()));
        }
    }
    out
}

// ------------------------------------------------------------
// Function: group_with
// Purpose: Drops `dragged` onto `target`. Two loose shows become
//          "New group". A show dropped on a grouped show is
//          inserted at that show. Two shows already in the same
//          group swap places instead of joining again.
// ------------------------------------------------------------
pub fn group_with(
    items: &mut Vec<FavoriteItem>,
    dragged: &str,
    target: &str,
) -> Option<GroupChange> {
    if dragged == target || locate(items, dragged).is_none() {
        return None;
    }
    match locate(items, target)? {
        Located::Grouped { group, index: to } => {
            let before = match locate(items, dragged)? {
                Located::Grouped {
                    group: from_group,
                    index: from_index,
                } if from_group == group => {
                    if from_index < to { to + 1 } else { to }
                }
                _ => to,
            };
            if !move_show(
                items,
                dragged,
                DropSlot::InGroup { group, before },
            ) {
                return None;
            }
            let group = match locate(items, target) {
                Some(Located::Grouped { group, .. }) => group,
                _ => return None,
            };
            Some(GroupChange::Joined(group))
        }
        Located::Loose(_) => {
            detach(items, dragged);
            let index = items.iter().position(
                |item| matches!(item, FavoriteItem::Show(feed_url) if feed_url == target),
            )?;
            items[index] = FavoriteItem::Group {
                name: "New group".to_string(),
                collapsed: false,
                shows: vec![target.to_string(), dragged.to_string()],
            };
            drop_empty(items);
            let index = items.iter().position(|item| {
                matches!(item, FavoriteItem::Group { shows, .. } if shows.iter().any(|feed| feed == target))
            })?;
            Some(GroupChange::Created(index))
        }
    }
}

// ------------------------------------------------------------
// Function: move_show
// Purpose: Moves one show to an insertion slot. Indices are the
//          ones from the list before the move.
// ------------------------------------------------------------
pub fn move_show(items: &mut Vec<FavoriteItem>, feed: &str, slot: DropSlot) -> bool {
    let Some(from) = locate(items, feed) else {
        return false;
    };
    if same_place(&from, slot) {
        return false;
    }
    if !detach(items, feed) {
        return false;
    }
    let slot = shift_slot(from, slot);
    if !insert_show(items, feed, slot) {
        items.push(FavoriteItem::Show(feed.to_string()));
    }
    drop_empty(items);
    true
}

// ------------------------------------------------------------
// Function: move_group
// Purpose: Reorders a top-level group. A slot inside another
//          group means just after that group.
// ------------------------------------------------------------
pub fn move_group(items: &mut Vec<FavoriteItem>, index: usize, slot: DropSlot) -> bool {
    let before = match slot {
        DropSlot::Before(before) => before,
        DropSlot::InGroup { group, .. } => group + 1,
    };
    if !matches!(items.get(index), Some(FavoriteItem::Group { .. })) {
        return false;
    }
    if index == before || index + 1 == before {
        return false;
    }
    let group = items.remove(index);
    let before = if before > index { before - 1 } else { before };
    let before = before.min(items.len());
    items.insert(before, group);
    true
}

pub fn move_group_by(items: &mut Vec<FavoriteItem>, index: usize, delta: isize) -> bool {
    if delta == 0 {
        return false;
    }
    let Some(target) = index.checked_add_signed(delta) else {
        return false;
    };
    if target >= items.len() {
        return false;
    }
    let before = if delta > 0 { target + 1 } else { target };
    move_group(items, index, DropSlot::Before(before))
}

pub fn rename_group(items: &mut [FavoriteItem], index: usize, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let Some(FavoriteItem::Group { name: slot, .. }) = items.get_mut(index) else {
        return false;
    };
    *slot = name.to_string();
    true
}

pub fn toggle_collapsed(items: &mut [FavoriteItem], index: usize) -> bool {
    let Some(FavoriteItem::Group { collapsed, .. }) = items.get_mut(index) else {
        return false;
    };
    *collapsed = !*collapsed;
    true
}

// ------------------------------------------------------------
// Function: ungroup
// Purpose: Replaces a group with its shows, in the same place.
// ------------------------------------------------------------
pub fn ungroup(items: &mut Vec<FavoriteItem>, index: usize) -> bool {
    let Some(FavoriteItem::Group { shows, .. }) = items.get(index) else {
        return false;
    };
    let shows = shows.clone();
    items.remove(index);
    for (offset, feed) in shows.into_iter().enumerate() {
        items.insert(index + offset, FavoriteItem::Show(feed));
    }
    true
}

pub fn remove_from_group(items: &mut Vec<FavoriteItem>, feed: &str) -> bool {
    let Some(Located::Grouped { group, .. }) = locate(items, feed) else {
        return false;
    };
    move_show(items, feed, DropSlot::Before(group + 1))
}

pub fn forget_show(items: &mut Vec<FavoriteItem>, feed: &str) {
    detach(items, feed);
    drop_empty(items);
}

pub fn group_index_of(items: &[FavoriteItem], feed: &str) -> Option<usize> {
    match locate(items, feed) {
        Some(Located::Grouped { group, .. }) => Some(group),
        _ => None,
    }
}

pub fn container_index(items: &[FavoriteItem], feed: &str) -> Option<usize> {
    match locate(items, feed)? {
        Located::Loose(index) | Located::Grouped { group: index, .. } => Some(index),
    }
}

fn locate(items: &[FavoriteItem], feed: &str) -> Option<Located> {
    for (index, item) in items.iter().enumerate() {
        match item {
            FavoriteItem::Show(feed_url) if feed_url == feed => {
                return Some(Located::Loose(index));
            }
            FavoriteItem::Group { shows, .. } => {
                if let Some(inner) = shows.iter().position(|feed_url| feed_url == feed) {
                    return Some(Located::Grouped {
                        group: index,
                        index: inner,
                    });
                }
            }
            _ => {}
        }
    }
    None
}

fn same_place(from: &Located, slot: DropSlot) -> bool {
    match (from, slot) {
        (Located::Loose(index), DropSlot::Before(before)) => {
            *index == before || *index + 1 == before
        }
        (
            Located::Grouped { group, index },
            DropSlot::InGroup {
                group: slot_group,
                before,
            },
        ) => *group == slot_group && (*index == before || *index + 1 == before),
        _ => false,
    }
}

fn detach(items: &mut Vec<FavoriteItem>, feed: &str) -> bool {
    for item in items.iter_mut() {
        if let FavoriteItem::Group { shows, .. } = item
            && let Some(index) = shows.iter().position(|feed_url| feed_url == feed)
        {
            shows.remove(index);
            return true;
        }
    }
    if let Some(index) = items
        .iter()
        .position(|item| matches!(item, FavoriteItem::Show(feed_url) if feed_url == feed))
    {
        items.remove(index);
        return true;
    }
    false
}

fn shift_slot(from: Located, slot: DropSlot) -> DropSlot {
    match (from, slot) {
        (Located::Loose(index), DropSlot::Before(before)) if before > index => {
            DropSlot::Before(before - 1)
        }
        (Located::Loose(index), DropSlot::InGroup { group, before }) if group > index => {
            DropSlot::InGroup {
                group: group - 1,
                before,
            }
        }
        (
            Located::Grouped { group, index },
            DropSlot::InGroup {
                group: slot_group,
                before,
            },
        ) if slot_group == group && before > index => DropSlot::InGroup {
            group: slot_group,
            before: before - 1,
        },
        (_, slot) => slot,
    }
}

fn insert_show(items: &mut Vec<FavoriteItem>, feed: &str, slot: DropSlot) -> bool {
    match slot {
        DropSlot::Before(index) => {
            let index = index.min(items.len());
            items.insert(index, FavoriteItem::Show(feed.to_string()));
            true
        }
        DropSlot::InGroup { group, before } => {
            let Some(FavoriteItem::Group { shows, .. }) = items.get_mut(group) else {
                return false;
            };
            let before = before.min(shows.len());
            shows.insert(before, feed.to_string());
            true
        }
    }
}

fn drop_empty(items: &mut Vec<FavoriteItem>) {
    items.retain(|item| match item {
        FavoriteItem::Group { shows, .. } => !shows.is_empty(),
        FavoriteItem::Show(_) => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loose(feeds: &[&str]) -> Vec<FavoriteItem> {
        feeds
            .iter()
            .map(|feed| FavoriteItem::Show((*feed).to_string()))
            .collect()
    }

    fn show(feed: &str) -> Show {
        Show {
            feed_url: feed.to_string(),
            title: feed.to_string(),
            author: String::new(),
            image_url: None,
            description: String::new(),
            episodes: Vec::new(),
        }
    }

    #[test]
    fn reconcile_drops_unknown_and_appends_new_favorites() {
        let items = vec![
            FavoriteItem::Show("gone".to_string()),
            FavoriteItem::Group {
                name: "News".to_string(),
                collapsed: false,
                shows: vec!["keep".to_string(), "also-gone".to_string()],
            },
        ];
        let favorites = vec![show("keep"), show("fresh")];
        let reconciled = reconcile(items, &favorites);
        assert_eq!(
            reconciled,
            vec![
                FavoriteItem::Group {
                    name: "News".to_string(),
                    collapsed: false,
                    shows: vec!["keep".to_string()],
                },
                FavoriteItem::Show("fresh".to_string()),
            ]
        );
    }

    #[test]
    fn group_with_two_loose_shows_starts_a_new_group() {
        let mut items = loose(&["a", "b", "c"]);
        let change = group_with(&mut items, "c", "a");
        assert_eq!(change, Some(GroupChange::Created(0)));
        assert_eq!(
            items,
            vec![
                FavoriteItem::Group {
                    name: "New group".to_string(),
                    collapsed: false,
                    shows: vec!["a".to_string(), "c".to_string()],
                },
                FavoriteItem::Show("b".to_string()),
            ]
        );
    }

    #[test]
    fn group_with_joins_an_existing_group() {
        let mut items = vec![
            FavoriteItem::Show("loose".to_string()),
            FavoriteItem::Group {
                name: "News".to_string(),
                collapsed: false,
                shows: vec!["a".to_string(), "b".to_string()],
            },
        ];
        let change = group_with(&mut items, "loose", "b");
        assert_eq!(change, Some(GroupChange::Joined(0)));
        assert_eq!(
            items,
            vec![FavoriteItem::Group {
                name: "News".to_string(),
                collapsed: false,
                shows: vec!["a".to_string(), "loose".to_string(), "b".to_string()],
            }]
        );
    }

    #[test]
    fn group_with_reorders_inside_the_same_group() {
        let mut items = vec![FavoriteItem::Group {
            name: "News".to_string(),
            collapsed: false,
            shows: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        }];
        assert_eq!(
            group_with(&mut items, "c", "a"),
            Some(GroupChange::Joined(0))
        );
        assert_eq!(group_shows(&items), vec!["c", "a", "b"]);
        assert_eq!(
            group_with(&mut items, "c", "b"),
            Some(GroupChange::Joined(0))
        );
        assert_eq!(group_shows(&items), vec!["a", "b", "c"]);
    }

    fn group_shows(items: &[FavoriteItem]) -> Vec<&str> {
        match &items[0] {
            FavoriteItem::Group { shows, .. } => shows.iter().map(String::as_str).collect(),
            FavoriteItem::Show(_) => Vec::new(),
        }
    }

    #[test]
    fn move_show_reorders_inside_and_across_groups() {
        let mut items = vec![
            FavoriteItem::Show("loose".to_string()),
            FavoriteItem::Group {
                name: "News".to_string(),
                collapsed: false,
                shows: vec!["a".to_string(), "b".to_string()],
            },
        ];
        assert!(move_show(
            &mut items,
            "loose",
            DropSlot::InGroup {
                group: 1,
                before: 1
            }
        ));
        assert_eq!(
            match &items[0] {
                FavoriteItem::Group { shows, .. } => shows.clone(),
                FavoriteItem::Show(_) => Vec::new(),
            },
            vec!["a".to_string(), "loose".to_string(), "b".to_string()]
        );

        assert!(move_show(&mut items, "b", DropSlot::Before(0)));
        assert_eq!(
            items,
            vec![
                FavoriteItem::Show("b".to_string()),
                FavoriteItem::Group {
                    name: "News".to_string(),
                    collapsed: false,
                    shows: vec!["a".to_string(), "loose".to_string()],
                },
            ]
        );

        assert!(move_show(
            &mut items,
            "loose",
            DropSlot::InGroup {
                group: 1,
                before: 0
            }
        ));
        assert_eq!(
            match &items[1] {
                FavoriteItem::Group { shows, .. } => shows.clone(),
                FavoriteItem::Show(_) => Vec::new(),
            },
            vec!["loose".to_string(), "a".to_string()]
        );
    }

    #[test]
    fn move_group_rename_and_ungroup() {
        let mut items = vec![
            FavoriteItem::Show("a".to_string()),
            FavoriteItem::Group {
                name: "News".to_string(),
                collapsed: true,
                shows: vec!["b".to_string(), "c".to_string()],
            },
            FavoriteItem::Show("d".to_string()),
        ];
        assert!(move_group(&mut items, 1, DropSlot::Before(0)));
        assert!(matches!(items[0], FavoriteItem::Group { .. }));
        assert!(rename_group(&mut items, 0, "  Morning  "));
        assert_eq!(
            match &items[0] {
                FavoriteItem::Group { name, .. } => name.as_str(),
                FavoriteItem::Show(_) => "",
            },
            "Morning"
        );
        assert!(ungroup(&mut items, 0));
        assert_eq!(items, loose(&["b", "c", "a", "d"]));
    }

    #[test]
    fn drop_band_splits_a_row_into_top_middle_and_bottom() {
        assert_eq!(drop_zone(0.0, 40.0, false), DropBand::Before);
        assert_eq!(drop_zone(5.0, 40.0, false), DropBand::Before);
        assert_eq!(drop_zone(20.0, 40.0, false), DropBand::On);
        assert_eq!(drop_zone(35.0, 40.0, false), DropBand::After);
        assert_eq!(drop_zone(5.0, 0.0, false), DropBand::On);
        assert_eq!(drop_zone(10.0, 40.0, true), DropBand::Before);
        assert_eq!(drop_zone(30.0, 40.0, true), DropBand::After);
    }

    #[test]
    fn drag_arms_after_the_threshold() {
        let mut gesture = DragGesture::press(DragItem::Show("a".to_string()), 10.0, 10.0);
        gesture.follow(12.0, 11.0);
        assert!(!gesture.armed);
        gesture.follow(15.0, 10.0);
        assert!(gesture.armed);
    }

    #[test]
    fn drag_arms_from_the_row_press_point() {
        // The origin is the point the row reported, not an older cursor.
        // Two pixels from that press stays a click. Five pixels arms it.
        let mut gesture = DragGesture::press(DragItem::Show("a".to_string()), 40.0, 80.0);
        gesture.follow(42.0, 80.0);
        assert!(!gesture.armed);
        gesture.follow(40.0, 85.0);
        assert!(gesture.armed);
    }

    #[test]
    fn dropping_on_the_end_of_the_list_removes_a_show_from_its_group() {
        let mut items = vec![
            FavoriteItem::Group {
                name: "News".to_string(),
                collapsed: false,
                shows: vec!["a".to_string(), "b".to_string()],
            },
            FavoriteItem::Show("loose".to_string()),
        ];
        let end = items.len();
        assert!(move_show(&mut items, "a", DropSlot::Before(end)));
        assert_eq!(
            items,
            vec![
                FavoriteItem::Group {
                    name: "News".to_string(),
                    collapsed: false,
                    shows: vec!["b".to_string()],
                },
                FavoriteItem::Show("loose".to_string()),
                FavoriteItem::Show("a".to_string()),
            ]
        );
    }

    #[test]
    fn dropping_a_group_inside_another_moves_it_after_that_group() {
        let mut items = vec![
            FavoriteItem::Group {
                name: "First".to_string(),
                collapsed: false,
                shows: vec!["a".to_string()],
            },
            FavoriteItem::Group {
                name: "Second".to_string(),
                collapsed: false,
                shows: vec!["b".to_string()],
            },
        ];
        assert!(!move_group(
            &mut items,
            0,
            DropSlot::InGroup {
                group: 0,
                before: 0
            }
        ));
        assert!(move_group(
            &mut items,
            0,
            DropSlot::InGroup {
                group: 1,
                before: 0
            }
        ));
        assert!(matches!(&items[0], FavoriteItem::Group { name, .. } if name == "Second"));
        assert!(matches!(&items[1], FavoriteItem::Group { name, .. } if name == "First"));
    }

    #[test]
    fn groups_round_trip() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "soundkeep-groups-{}-{nanos}.json",
            std::process::id()
        ));
        let items = vec![FavoriteItem::Group {
            name: "News".to_string(),
            collapsed: true,
            shows: vec!["https://example.com/feed.xml".to_string()],
        }];
        save_to(&path, &items).unwrap();
        assert_eq!(load_from(&path), items);
        let _ = fs::remove_file(path);
    }
}
