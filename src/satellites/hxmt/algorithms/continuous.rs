use super::super::types::Hxmt;
use crate::{search::trigger::Trigger, types::Span};

pub fn continuous(
    triggers: Vec<Trigger<Hxmt>>,
    interval: Span<Hxmt>,
    duration: Span<Hxmt>,
    count: i32,
) -> Vec<Trigger<Hxmt>> {
    if triggers.is_empty() {
        return triggers;
    }
    let mut veto = vec![false; triggers.len()];
    let mut last_time = triggers[0].start;
    let mut begin = 0;
    for i in 1..triggers.len() {
        let time = triggers[i].start;
        if (time - last_time) > interval || i == triggers.len() - 1 {
            if ((last_time - triggers[begin].start) > duration) || i - begin >= count as usize {
                veto[begin..i].fill(true);
            }
            begin = i;
        }
        last_time = time;
    }
    veto.into_iter()
        .zip(triggers)
        .filter(|(c, _)| !(*c))
        .map(|(_, t)| t)
        .collect()
}
