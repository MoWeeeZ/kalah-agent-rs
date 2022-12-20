use rand::{thread_rng, Rng};

#[allow(dead_code)]
pub fn softmax(nums: &[f32], beta: f32) -> Vec<f32> {
    let mut num_vec = Vec::new();
    num_vec.extend_from_slice(nums);

    let nums: &mut [f32] = &mut num_vec;

    let max_num = *nums.iter().max_by(|&x, &y| x.partial_cmp(y).unwrap()).unwrap();

    for num in nums.iter_mut() {
        *num -= max_num;
        *num *= beta;
        *num = num.exp()
    }

    let sum: f32 = nums.iter().sum();

    for num in nums.iter_mut() {
        *num /= sum;
    }

    num_vec
}

#[allow(dead_code)]
pub fn sample_index_weighted(weights: &[f32]) -> usize {
    assert!(!weights.is_empty(), "Trying to sample from emptry distribution");

    // shortcut if there if only 1 element to sample from
    if weights.len() == 1 {
        assert!(weights[0] != 0.0, "Only weight is zero");
        return 0;
    }

    // Efraimidis-Spirakis sampling
    let mut rng = thread_rng();
    let roll_outs = weights.iter().map(|w| rng.gen::<f32>().powf(1.0 / w));

    roll_outs
        .enumerate()
        .min_by(|(_, r1), (_, r2)| r1.partial_cmp(r2).unwrap())
        .unwrap()
        .0
}
