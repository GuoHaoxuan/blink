use crate::types::HxmtHe;
use blink_algorithms::types::Candidate;

pub fn continuous(
    triggers: Vec<Candidate<HxmtHe>>,
    interval: uom::si::f64::Time,
    duration: uom::si::f64::Time,
    count: i32,
) -> Vec<Candidate<HxmtHe>> {
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
