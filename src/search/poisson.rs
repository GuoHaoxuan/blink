pub fn poisson_isf(p: f64, lambda: f64) -> u32 {
    let mut k = 0;
    let mut cumulative_prob = (-lambda).exp();
    let mut part = 0.0;

    while cumulative_prob < 1.0 - p {
        k += 1;
        part += (lambda / k as f64).ln();
        cumulative_prob += (-lambda + part).exp();
    }

    k
}

pub fn poisson_isf_cached(p: f64, lambda: f64, cache: &mut [u32]) -> u32 {
    let lambda_100x = (lambda * 100.0).round() as usize;
    if lambda_100x == 0 {
        return 0;
    }
    if lambda_100x >= cache.len() {
        return poisson_isf(p, lambda);
    }
    if cache[lambda_100x] == 0 {
        cache[lambda_100x] = poisson_isf(p, lambda);
    }
    cache[lambda_100x]
}
