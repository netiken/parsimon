use parsimon_core::network::Flow;

use crate::utils;

// PRECONDITION: flows in `a` and `b` are sorted by start time
pub(crate) fn max_wmape_xs(a: &[Flow], b: &[Flow]) -> f64 {
    let s_a = utils::percentiles_100(a, |f| f.size);
    let s_b = utils::percentiles_100(b, |f| f.size);
    let s_mae = utils::wmape(
        &s_a.iter().map(|x| x.into_f64()).collect::<Vec<_>>(),
        &s_b.iter().map(|x| x.into_f64()).collect::<Vec<_>>(),
    );
    let d_a = utils::percentiles_100(&utils::deltas(a), |&x| x);
    let d_b = utils::percentiles_100(&utils::deltas(b), |&x| x);
    let d_mae = utils::wmape(
        &d_a.iter().map(|x| x.into_f64()).collect::<Vec<_>>(),
        &d_b.iter().map(|x| x.into_f64()).collect::<Vec<_>>(),
    );
    std::cmp::max_by(s_mae, d_mae, |x, y| {
        x.partial_cmp(y)
            .expect("`max_mae_xs`: failed to compare floats")
    })
}
