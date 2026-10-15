// ------------------------------------------------------------
// Module: queue tests
// Purpose: Playback order, including play-next while shuffled.
// ------------------------------------------------------------

use super::*;

fn sample(count: usize) -> Queue {
    let mut queue = Queue::new();
    let tracks = (0..count)
        .map(|index| Track::placeholder(&format!("track-{index}.mp3")))
        .collect();
    queue.push_many(tracks);
    queue
}

fn walk_to(queue: &mut Queue, display_index: usize) {
    let guard = queue.len() + 1;
    let mut steps = 0;
    while queue.current_display_index() != Some(display_index) {
        assert!(matches!(queue.go_next(false), Step::Play));
        steps += 1;
        assert!(steps <= guard, "could not reach {display_index}");
    }
}

#[test]
fn shuffle_covers_every_track_once() {
    let mut queue = sample(6);
    queue.set_shuffle(true);
    let mut seen = vec![queue.current_display_index().unwrap()];
    for _ in 0..5 {
        assert!(matches!(queue.go_next(true), Step::Play));
        seen.push(queue.current_display_index().unwrap());
    }
    assert!(matches!(queue.go_next(true), Step::End));
    seen.sort();
    seen.dedup();
    assert_eq!(seen, (0..6).collect::<Vec<_>>());
}

#[test]
fn repeat_all_wraps_and_repeat_one_restarts_only_when_automatic() {
    let mut queue = sample(4);
    queue.set_repeat(RepeatMode::All);
    walk_to(&mut queue, 3);
    assert!(matches!(queue.go_next(true), Step::Play));
    assert_eq!(queue.current_display_index(), Some(0));

    queue.set_repeat(RepeatMode::One);
    assert!(matches!(queue.go_next(true), Step::Restart));
    assert_eq!(queue.current_display_index(), Some(0));
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current_display_index(), Some(1));
}

#[test]
fn next_and_previous_at_the_edges() {
    let mut queue = sample(3);
    assert!(matches!(
        queue.go_previous(Duration::from_secs(1)),
        Step::Restart
    ));
    assert_eq!(queue.current_display_index(), Some(0));

    walk_to(&mut queue, 2);
    assert!(matches!(queue.go_next(true), Step::End));
    assert_eq!(queue.current_display_index(), Some(2));

    queue.set_repeat(RepeatMode::All);
    queue.select_display(0);
    assert!(matches!(queue.go_previous(Duration::ZERO), Step::Play));
    assert_eq!(queue.current_display_index(), Some(2));
}

#[test]
fn previous_after_three_seconds_restarts_the_same_track() {
    let mut queue = sample(3);
    walk_to(&mut queue, 1);
    assert!(matches!(
        queue.go_previous(Duration::from_secs(4)),
        Step::Restart
    ));
    assert_eq!(queue.current_display_index(), Some(1));
    assert!(matches!(
        queue.go_previous(Duration::from_secs(3)),
        Step::Play
    ));
    assert_eq!(queue.current_display_index(), Some(0));
}

#[test]
fn play_next_without_shuffle_slots_after_the_cursor() {
    let mut queue = sample(4);
    assert!(queue.play_next(3));
    assert_eq!(queue.current_display_index(), Some(0));
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current_display_index(), Some(3));
    assert_eq!(queue.tracks().len(), 4);

    let mut queue = sample(4);
    walk_to(&mut queue, 2);
    assert!(queue.play_next(0));
    assert_eq!(queue.current_display_index(), Some(2));
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current_display_index(), Some(0));
}

#[test]
fn play_next_with_shuffle_ignores_display_order() {
    let mut queue = sample(6);
    queue.set_shuffle(true);
    let current = queue.current_display_index().unwrap();
    let target = (0..6).find(|index| *index != current).unwrap();
    assert!(queue.play_next(target));
    assert_eq!(queue.current_display_index(), Some(current));
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current_display_index(), Some(target));
    let titles: Vec<_> = queue
        .tracks()
        .iter()
        .map(|track| track.title.clone())
        .collect();
    assert_eq!(
        titles,
        (0..6)
            .map(|index| format!("track-{index}"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn insert_next_plays_before_the_rest_even_when_shuffled() {
    let mut queue = sample(3);
    queue.select_display(1);
    queue.insert_next(Track::placeholder("episode.mp3"));
    assert_eq!(queue.tracks().len(), 4);
    assert_eq!(queue.current_display_index(), Some(1));
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current().unwrap().title, "episode");

    let mut queue = sample(5);
    queue.set_shuffle(true);
    let current = queue.current_display_index();
    queue.insert_next(Track::placeholder("episode.mp3"));
    assert_eq!(queue.current_display_index(), current);
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current().unwrap().title, "episode");
}

#[test]
fn turning_shuffle_off_keeps_sequential_neighbors() {
    let mut queue = sample(4);
    queue.select_display(2);
    queue.set_shuffle(true);
    queue.set_shuffle(false);
    assert_eq!(queue.current_display_index(), Some(2));
    assert!(matches!(queue.go_next(false), Step::Play));
    assert_eq!(queue.current_display_index(), Some(3));
}
