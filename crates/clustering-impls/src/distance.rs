use parsimon_core::network::Flow;

use crate::utils;

// PRECONDITION: flows in `a` and `b`, if any, are sorted by start time
pub fn max_wmape_xs(a: &[Flow], b: &[Flow]) -> f64 {
    match (a.len() < 2, b.len() < 2) {
        (true, true) => 0.0,
        (true, _) => f64::MAX,
        (_, true) => f64::MAX,
        _ => {
            let s_a = utils::percentiles(a, |f| f.size);
            let s_b = utils::percentiles(b, |f| f.size);
            let s_wmape = utils::wmape(&s_a, &s_b);
            let d_a = utils::percentiles(&utils::deltas(a), |&x| x);
            let d_b = utils::percentiles(&utils::deltas(b), |&x| x);
            let d_wmape = utils::wmape(&d_a, &d_b);
            std::cmp::max_by(s_wmape, d_wmape, |x, y| {
                x.partial_cmp(y)
                    .expect("`max_wmape_xs`: failed to compare floats")
            })
        }
    }
}

// PRECONDITION: flows in `a` and `b` are sorted by start time
// pub fn max_mae_rescaled_xs(a: &[Flow], b: &[Flow]) -> f64 {
//     let s_a = utils::percentiles(a, |f| f.size);
//     let s_b = utils::percentiles(b, |f| f.size);
//     let s_mae = utils::mae(&utils::rescale(&s_a), &utils::rescale(&s_b));
//     let d_a = utils::percentiles(&utils::deltas(a), |&x| x);
//     let d_b = utils::percentiles(&utils::deltas(b), |&x| x);
//     let d_mae = utils::mae(&utils::rescale(&d_a), &utils::rescale(&d_b));
//     std::cmp::max_by(s_mae, d_mae, |x, y| {
//         x.partial_cmp(y)
//             .expect("`max_wmape_xs`: failed to compare floats")
//     })
// }
