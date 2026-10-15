// ------------------------------------------------------------
// Module: layout
// Purpose: The three podcast panes and the saved split tree.
//          A missing or broken file falls back to the default.
// ------------------------------------------------------------

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use iced::widget::pane_grid::{self, Axis, Configuration, Node};
use serde::{Deserialize, Serialize};

use crate::config;

// ------------------------------------------------------------
// Enum: PodcastPane
// Purpose: Which tool a pane is showing. The grid can move them;
//          it cannot create a second copy of one.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PodcastPane {
    Favorites,
    Search,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SavedNode {
    Split {
        axis: SavedAxis,
        ratio: f32,
        a: Box<SavedNode>,
        b: Box<SavedNode>,
    },
    Pane(PodcastPane),
}

pub fn load_panes() -> pane_grid::State<PodcastPane> {
    panes_from_path(&config::layout_path())
}

pub fn save_panes(state: &pane_grid::State<PodcastPane>) -> std::io::Result<()> {
    let Some(saved) = capture(state) else {
        return Ok(());
    };
    save_to(&config::layout_path(), &saved)
}

fn panes_from_path(path: &Path) -> pane_grid::State<PodcastPane> {
    let Ok(text) = fs::read_to_string(path) else {
        return default_panes();
    };
    panes_from_json(&text)
}

pub fn panes_from_json(text: &str) -> pane_grid::State<PodcastPane> {
    let Ok(saved) = serde_json::from_str::<SavedNode>(text) else {
        return default_panes();
    };
    if !is_valid(&saved) {
        return default_panes();
    }
    pane_grid::State::with_configuration(to_configuration(saved))
}

pub fn default_panes() -> pane_grid::State<PodcastPane> {
    pane_grid::State::with_configuration(to_configuration(default_saved()))
}

// ------------------------------------------------------------
// Function: default_saved
// Purpose: Favorites over Search on the left, episodes on the right.
// ------------------------------------------------------------
pub fn default_saved() -> SavedNode {
    SavedNode::Split {
        axis: SavedAxis::Vertical,
        ratio: 0.32,
        a: Box::new(SavedNode::Split {
            axis: SavedAxis::Horizontal,
            ratio: 0.6,
            a: Box::new(SavedNode::Pane(PodcastPane::Favorites)),
            b: Box::new(SavedNode::Pane(PodcastPane::Search)),
        }),
        b: Box::new(SavedNode::Pane(PodcastPane::Detail)),
    }
}

// ------------------------------------------------------------
// Function: capture
// Purpose: Reads the live split tree back into the saved form.
// ------------------------------------------------------------
pub fn capture(state: &pane_grid::State<PodcastPane>) -> Option<SavedNode> {
    node_to_saved(state.layout(), state)
}

fn node_to_saved(node: &Node, state: &pane_grid::State<PodcastPane>) -> Option<SavedNode> {
    match node {
        Node::Split {
            axis, ratio, a, b, ..
        } => Some(SavedNode::Split {
            axis: match axis {
                Axis::Horizontal => SavedAxis::Horizontal,
                Axis::Vertical => SavedAxis::Vertical,
            },
            ratio: *ratio,
            a: Box::new(node_to_saved(a, state)?),
            b: Box::new(node_to_saved(b, state)?),
        }),
        Node::Pane(pane) => state.get(*pane).copied().map(SavedNode::Pane),
    }
}

fn is_valid(node: &SavedNode) -> bool {
    let mut panes = Vec::new();
    if !collect(node, &mut panes) {
        return false;
    }
    let unique: BTreeSet<_> = panes.iter().copied().collect();
    unique.len() == 3
        && unique.contains(&PodcastPane::Favorites)
        && unique.contains(&PodcastPane::Search)
        && unique.contains(&PodcastPane::Detail)
}

fn collect(node: &SavedNode, panes: &mut Vec<PodcastPane>) -> bool {
    match node {
        SavedNode::Pane(kind) => {
            panes.push(*kind);
            true
        }
        SavedNode::Split { ratio, a, b, .. } => {
            ratio.is_finite()
                && (0.0..1.0).contains(ratio)
                && collect(a, panes)
                && collect(b, panes)
        }
    }
}

fn to_configuration(node: SavedNode) -> Configuration<PodcastPane> {
    match node {
        SavedNode::Pane(kind) => Configuration::Pane(kind),
        SavedNode::Split { axis, ratio, a, b } => Configuration::Split {
            axis: match axis {
                SavedAxis::Horizontal => Axis::Horizontal,
                SavedAxis::Vertical => Axis::Vertical,
            },
            ratio,
            a: Box::new(to_configuration(*a)),
            b: Box::new(to_configuration(*b)),
        },
    }
}

fn save_to(path: &Path, node: &SavedNode) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(node).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_round_trips() {
        let state = default_panes();
        let saved = capture(&state).expect("default panes");
        assert_eq!(saved, default_saved());
        let again = capture(&pane_grid::State::with_configuration(to_configuration(
            saved.clone(),
        )))
        .expect("restored panes");
        assert_eq!(again, saved);
    }

    #[test]
    fn corrupt_layout_uses_the_default() {
        let state = panes_from_json("{not json");
        assert_eq!(capture(&state), Some(default_saved()));
        let duplicate = r#"{"Pane":"Favorites"}"#;
        let state = panes_from_json(duplicate);
        assert_eq!(capture(&state), Some(default_saved()));
    }
}
