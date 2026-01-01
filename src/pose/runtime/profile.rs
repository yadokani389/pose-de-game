use crate::infer::InferenceTimings;

#[derive(Default)]
pub(super) struct ProfileStats {
    sample_count: u32,
    sum_preprocess_ms: f64,
    sum_pose_ms: f64,
    sum_seg_ms: f64,
    sum_postprocess_ms: f64,
    sum_total_ms: f64,
}

pub(super) fn update_profile(profile: &mut ProfileStats, timings: InferenceTimings, total_ms: f64) {
    profile.sample_count += 1;
    profile.sum_preprocess_ms += timings.preprocess_ms;
    profile.sum_pose_ms += timings.pose_infer_ms;
    profile.sum_seg_ms += timings.seg_infer_ms;
    profile.sum_postprocess_ms += timings.postprocess_ms;
    profile.sum_total_ms += total_ms;
}

pub(super) fn log_profile(profile: &mut ProfileStats) {
    if profile.sample_count == 0 {
        return;
    }
    let denom = profile.sample_count as f64;
    println!(
        "profile: pre={:.2} pose={:.2} seg={:.2} post={:.2} total={:.2}",
        profile.sum_preprocess_ms / denom,
        profile.sum_pose_ms / denom,
        profile.sum_seg_ms / denom,
        profile.sum_postprocess_ms / denom,
        profile.sum_total_ms / denom
    );
    profile.sample_count = 0;
    profile.sum_preprocess_ms = 0.0;
    profile.sum_pose_ms = 0.0;
    profile.sum_seg_ms = 0.0;
    profile.sum_postprocess_ms = 0.0;
    profile.sum_total_ms = 0.0;
}
