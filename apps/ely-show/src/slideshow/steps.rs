//! Where a show stands, and where each key takes it.

use ely_lok::show::{Effect, ShowSlide};

use crate::preview::timeline::{Run, length, schedule};

/// Effects in click groups; group 0 runs on entry.
pub fn groups(effects: &[Effect]) -> Vec<Vec<Effect>> {
    let mut groups = vec![Vec::new()];
    for effect in effects {
        if effect.start == "click" {
            groups.push(Vec::new());
        }
        groups.last_mut().expect("one group").push(effect.clone());
    }
    groups
}

/// A slide and its clicks done, or the end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    Slide { part: usize, clicks: usize },
    End,
}

/// The next place on a click.
pub fn next(slides: &[ShowSlide], place: Place) -> Place {
    match place {
        Place::Slide { part, clicks } if clicks + 1 < groups(&slides[part].effects).len() => {
            Place::Slide {
                part,
                clicks: clicks + 1,
            }
        }
        Place::Slide { part, .. } => after(slides, part),
        Place::End => Place::End,
    }
}

/// The first shown slide after `part`, or the end.
pub fn after(slides: &[ShowSlide], part: usize) -> Place {
    (part + 1..slides.len())
        .find(|at| !slides[*at].hidden)
        .map_or(Place::End, |part| Place::Slide { part, clicks: 0 })
}

/// The previous slide, fully built; the first stays.
pub fn previous(slides: &[ShowSlide], place: Place) -> Place {
    let from = match place {
        Place::Slide { part, .. } => part,
        Place::End => slides.len(),
    };
    let built = |part: usize| Place::Slide {
        part,
        clicks: groups(&slides[part].effects).len() - 1,
    };
    match (0..from).rev().find(|at| !slides[*at].hidden) {
        Some(part) => built(part),
        None => place,
    }
}

/// Runs from entry: group `n` starts at `started[n]` seconds.
pub fn runs(effects: &[Effect], started: &[f64]) -> Vec<Run> {
    (groups(effects).iter().zip(started))
        .flat_map(|(group, at)| {
            schedule(group).into_iter().map(move |run| Run {
                begin: run.begin + at,
                end: run.end + at,
                ..run
            })
        })
        .collect()
}

/// A click group starts now or once all before end.
pub fn click_at(effects: &[Effect], clicks: &[f64], now: f64) -> f64 {
    let before = clicks.iter().copied().fold(now, f64::max);
    length(&runs(effects, clicks)).max(before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ely_lok::show::Transition;

    fn effect(start: &str) -> Effect {
        Effect {
            preset: "ooo-entrance-fade-in".into(),
            label: String::new(),
            class: "entrance".into(),
            target: Some(0),
            hash: start.into(),
            start: start.into(),
            duration: 1.0,
            delay: 0.0,
        }
    }

    fn slide(hidden: bool, effects: Vec<Effect>) -> ShowSlide {
        ShowSlide {
            hidden,
            notes: String::new(),
            transition: Transition {
                preset: String::new(),
                duration: 0.0,
                auto: false,
                time: 0.0,
            },
            effects,
            media: Vec::new(),
        }
    }

    #[test]
    fn with_and_after_join_the_click_before() {
        let sizes = |effects: &[Effect]| groups(effects).iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!(sizes(&[]), [0]);
        assert_eq!(sizes(&[effect("with"), effect("click")]), [1, 1]);
        assert_eq!(sizes(&[effect("click"), effect("after"), effect("click")]), [0, 2, 1]);
    }

    #[test]
    fn clicks_play_effects_then_skip_hidden_slides_to_the_end() {
        let deck = [
            slide(false, vec![effect("click")]),
            slide(true, Vec::new()),
            slide(false, Vec::new()),
        ];
        let mut place = Place::Slide { part: 0, clicks: 0 };
        let mut seen = Vec::new();
        for _ in 0..4 {
            place = next(&deck, place);
            seen.push(place);
        }
        assert_eq!(
            seen,
            [
                Place::Slide { part: 0, clicks: 1 },
                Place::Slide { part: 2, clicks: 0 },
                Place::End,
                Place::End
            ]
        );
        assert_eq!(previous(&deck, Place::End), Place::Slide { part: 2, clicks: 0 });
        assert_eq!(
            previous(&deck, Place::Slide { part: 2, clicks: 0 }),
            Place::Slide { part: 0, clicks: 1 },
            "back to a built slide, past the hidden one"
        );
        let first = Place::Slide { part: 0, clicks: 0 };
        assert_eq!(previous(&deck, first), first);
    }

    #[test]
    fn a_quick_click_waits_for_the_groups_and_transition_before() {
        let long = |start| Effect {
            duration: 10.0,
            ..effect(start)
        };
        let effects = [long("click"), long("click")];
        assert_eq!(click_at(&effects, &[0.0], 0.1), 0.1, "nothing runs yet");
        assert_eq!(click_at(&effects, &[0.0, 0.1], 0.2), 10.1, "after the first");
        assert_eq!(click_at(&effects, &[2.0], 0.5), 2.0, "after the transition");
    }

    #[test]
    fn each_group_runs_from_its_own_click() {
        let effects = [effect("with"), effect("click"), effect("after")];
        let times: Vec<(f64, f64)> = runs(&effects, &[0.0, 5.0])
            .iter()
            .map(|r| (r.begin, r.end))
            .collect();
        assert_eq!(times, [(0.0, 1.0), (5.0, 6.0), (6.0, 7.0)]);
    }
}
