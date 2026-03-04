//! Deterministic simulation systems and tick loop.

pub mod phase_wheel;
pub mod scheduling;
pub mod systems;
pub mod tick;

#[cfg(test)]
mod determinism_guards {
    fn assert_no_float_tokens(path: &str, src: &str) {
        let forbidden = ["f32", "f64", "as f32", "as f64"];
        for token in forbidden {
            assert!(
                !src.contains(token),
                "determinism guard failed: found `{}` in {}",
                token,
                path
            );
        }
    }

    #[test]
    fn tick_path_systems_use_integer_or_fixed_point_only() {
        let files = [
            (
                "sim/systems/construction.rs",
                include_str!("systems/construction.rs"),
            ),
            ("sim/systems/buildings.rs", include_str!("systems/buildings.rs")),
            ("sim/systems/utilities.rs", include_str!("systems/utilities.rs")),
            ("sim/systems/population.rs", include_str!("systems/population.rs")),
            ("sim/systems/jobs.rs", include_str!("systems/jobs.rs")),
            ("sim/systems/transport.rs", include_str!("systems/transport.rs")),
            ("sim/systems/finance.rs", include_str!("systems/finance.rs")),
            (
                "sim/systems/city_events.rs",
                include_str!("systems/city_events.rs"),
            ),
            ("sim/tick.rs", include_str!("tick.rs")),
        ];
        for (path, src) in files {
            assert_no_float_tokens(path, src);
        }
    }
}
