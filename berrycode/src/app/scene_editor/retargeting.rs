#![allow(dead_code)]
//! Animation retargeting: transfer animation from one humanoid skeleton to another.

use super::humanoid_avatar::HumanoidAvatar;

// ---------------------------------------------------------------------------
// Quaternion helpers (xyzw layout: [x, y, z, w])
// ---------------------------------------------------------------------------

/// Multiply two quaternions: result = a * b.
pub fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// Inverse (conjugate) of a unit quaternion.
pub fn quat_inverse(q: [f32; 4]) -> [f32; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

/// Normalize a quaternion to unit length.
pub fn quat_normalize(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

// ---------------------------------------------------------------------------
// Retargeting context
// ---------------------------------------------------------------------------

/// Context for retargeting animations between two humanoid avatars.
pub struct RetargetingContext {
    pub source: HumanoidAvatar,
    pub target: HumanoidAvatar,
}

impl RetargetingContext {
    pub fn new(source: HumanoidAvatar, target: HumanoidAvatar) -> Self {
        Self { source, target }
    }
}

/// Retarget a rotation from source skeleton to target skeleton using quaternion
/// delta retargeting.
///
/// The idea: compute the delta rotation relative to the source bind pose, then
/// apply that same delta on top of the target bind pose.
///
/// `target_rot = target_bind * inverse(source_bind) * source_anim`
///
/// All quaternions are in [x, y, z, w] format.
pub fn retarget_rotation(
    source_bind: [f32; 4],
    source_anim: [f32; 4],
    target_bind: [f32; 4],
) -> [f32; 4] {
    // delta = inverse(source_bind) * source_anim
    let delta = quat_mul(quat_inverse(source_bind), source_anim);
    // result = target_bind * delta
    let result = quat_mul(target_bind, delta);
    quat_normalize(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: [f32; 4], b: [f32; 4], eps: f32) -> bool {
        // Quaternions q and -q represent the same rotation
        let same = a.iter().zip(&b).all(|(x, y)| (x - y).abs() < eps);
        let neg = a.iter().zip(&b).all(|(x, y)| (x + y).abs() < eps);
        same || neg
    }

    #[test]
    fn quat_mul_identity() {
        let id = [0.0, 0.0, 0.0, 1.0];
        let q = [0.1, 0.2, 0.3, 0.9];
        let q_norm = quat_normalize(q);
        let result = quat_mul(id, q_norm);
        assert!(approx_eq(result, q_norm, 1e-5));
    }

    #[test]
    fn quat_mul_inverse_gives_identity() {
        let q = quat_normalize([0.1, 0.2, 0.3, 0.9]);
        let inv = quat_inverse(q);
        let result = quat_mul(q, inv);
        let id = [0.0, 0.0, 0.0, 1.0];
        assert!(approx_eq(result, id, 1e-5));
    }

    #[test]
    fn quat_normalize_unit() {
        let q = [3.0, 0.0, 0.0, 4.0]; // length 5
        let n = quat_normalize(q);
        assert!((n[0] - 0.6).abs() < 1e-5);
        assert!((n[3] - 0.8).abs() < 1e-5);
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] + n[3] * n[3]).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
    }

    #[test]
    fn quat_normalize_zero_gives_identity() {
        let q = [0.0, 0.0, 0.0, 0.0];
        let n = quat_normalize(q);
        assert_eq!(n, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn retarget_same_bind_pose() {
        // If source and target have the same bind pose, retargeted rotation
        // should equal the source animation rotation.
        let bind = quat_normalize([0.0, 0.0, 0.3826834, 0.9238795]); // 45 deg Z
        let anim = quat_normalize([0.0, 0.0, 0.7071068, 0.7071068]); // 90 deg Z
        let result = retarget_rotation(bind, anim, bind);
        assert!(approx_eq(result, anim, 1e-4));
    }

    #[test]
    fn retarget_identity_anim_gives_target_bind() {
        // If the animation rotation equals the source bind pose (no movement),
        // the result should be the target bind pose.
        let source_bind = quat_normalize([0.1, 0.0, 0.0, 0.99]);
        let target_bind = quat_normalize([0.0, 0.2, 0.0, 0.98]);
        let result = retarget_rotation(source_bind, source_bind, target_bind);
        assert!(approx_eq(result, target_bind, 1e-4));
    }
}
