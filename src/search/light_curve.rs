pub fn light_curve(time: &[f64], start: f64, stop: f64, bin_size: f64) -> Vec<u32> {
    let length = ((stop - start) / bin_size).ceil() as usize;
    let mut light_curve = vec![0; length];
    time.iter().for_each(|&time| {
        if time >= start && time < stop {
            let index = ((time - start) / bin_size).floor() as usize;
            light_curve[index] += 1;
        }
    });
    light_curve
}

pub fn prefix_sum(light_curve: &[u32]) -> Vec<u32> {
    light_curve
        .iter()
        .scan(0, |state, &x| {
            *state += x;
            Some(*state)
        })
        .collect()
}
