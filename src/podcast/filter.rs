// ------------------------------------------------------------
// Module: filter
// Purpose: Matches an episode title against the words typed in
//          the filter. A word can be unfinished or one letter off,
//          and closer titles sort first.
// ------------------------------------------------------------

const EXACT: u32 = 100;
const PREFIX: u32 = 60;
const TYPO: u32 = 40;
const ADJACENT: u32 = 80;

// ------------------------------------------------------------
// Function: query_words
// Purpose: The words the person typed, lowercased, with
//          punctuation dropped. An empty filter has no words.
// ------------------------------------------------------------
pub fn query_words(filter: &str) -> Vec<String> {
    words(filter)
}

// ------------------------------------------------------------
// Function: match_score
// Purpose: How well the title matches every typed word.
//          None hides the episode. A higher score is a better match.
// ------------------------------------------------------------
pub fn match_score(query: &[String], title: &str) -> Option<u32> {
    if query.is_empty() {
        return None;
    }
    let title_words = words(title);
    if title_words.is_empty() {
        return None;
    }
    let mut used = vec![false; title_words.len()];
    let mut chosen = Vec::with_capacity(query.len());
    best(query, &title_words, 0, &mut used, &mut chosen).map(|(score, _)| score)
}

fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn best(
    query: &[String],
    title_words: &[String],
    index: usize,
    used: &mut [bool],
    chosen: &mut Vec<usize>,
) -> Option<(u32, Vec<usize>)> {
    if index == query.len() {
        return Some((adjacency(chosen), chosen.clone()));
    }
    let mut winner: Option<(u32, Vec<usize>)> = None;
    for (title_index, title_word) in title_words.iter().enumerate() {
        if used[title_index] {
            continue;
        }
        let Some(word_score) = word_score(&query[index], title_word) else {
            continue;
        };
        used[title_index] = true;
        chosen.push(title_index);
        if let Some((rest, assignment)) = best(query, title_words, index + 1, used, chosen) {
            let total = word_score + rest;
            let better = winner.as_ref().is_none_or(|(score, _)| total > *score);
            if better {
                winner = Some((total, assignment));
            }
        }
        chosen.pop();
        used[title_index] = false;
    }
    winner
}

fn adjacency(chosen: &[usize]) -> u32 {
    chosen
        .windows(2)
        .filter(|pair| pair[1] == pair[0] + 1)
        .count() as u32
        * ADJACENT
}

fn word_score(query: &str, title_word: &str) -> Option<u32> {
    if query == title_word {
        return Some(EXACT);
    }
    if title_word.starts_with(query) {
        return Some(PREFIX);
    }
    if query.chars().count() >= 4 && within_one(query, title_word) {
        return Some(TYPO);
    }
    None
}

fn within_one(left: &str, right: &str) -> bool {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let (shorter, longer) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    if longer.len() - shorter.len() > 1 {
        return false;
    }
    if shorter.len() == longer.len() {
        return shorter
            .iter()
            .zip(longer.iter())
            .filter(|(a, b)| a != b)
            .count()
            <= 1;
    }
    let mut short_index = 0;
    let mut long_index = 0;
    let mut skipped = false;
    while short_index < shorter.len() && long_index < longer.len() {
        if shorter[short_index] == longer[long_index] {
            short_index += 1;
            long_index += 1;
        } else if !skipped {
            skipped = true;
            long_index += 1;
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASON: &str = "Bitcoin vs Wall Street | American HODL, Junseth & Erik Cason";
    const YAKES: &str = "Bitcoin's Next Rally Could Break the Cycle | Eric Yakes";

    #[test]
    fn eric_c_finds_erik_cason() {
        let query = query_words("Eric C");
        assert!(match_score(&query, CASON).is_some());
    }

    #[test]
    fn eric_does_not_match_a_title_that_only_contains_american() {
        let query = query_words("eric");
        assert!(match_score(&query, "The American Dream").is_none());
    }

    #[test]
    fn eric_c_ranks_erik_cason_above_eric_and_cycle() {
        let query = query_words("Eric C");
        let cason = match_score(&query, CASON).unwrap();
        let yakes = match_score(&query, YAKES).unwrap();
        assert!(cason > yakes);
    }

    #[test]
    fn an_exact_name_ranks_above_one_typo() {
        let query = query_words("eric yakes");
        let exact = match_score(&query, "Eric Yakes").unwrap();
        let typo = match_score(&query, "Erik Yakes").unwrap();
        assert!(exact > typo);
    }

    #[test]
    fn short_words_do_not_match_by_typo() {
        let query = query_words("al");
        assert!(match_score(&query, "a").is_none());
        assert!(match_score(&query, "el").is_none());
        assert!(match_score(&query_words("cat"), "car").is_none());
        // "al" is the start of "all", so it still matches while it is being typed.
        assert!(match_score(&query, "all").is_some());
    }

    #[test]
    fn punctuation_is_ignored_and_an_empty_filter_has_no_words() {
        assert!(query_words("").is_empty());
        assert!(query_words("   ").is_empty());
        assert_eq!(query_words("Eric, C!"), vec!["eric".to_string(), "c".to_string()]);
        assert!(match_score(&query_words("eric,"), "Eric!").is_some());
    }
}
