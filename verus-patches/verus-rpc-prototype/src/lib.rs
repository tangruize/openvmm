//! Stage 1 prototype: passing fractional ghost ownership through an RPC.

#[allow(unused_imports)]
use vstd::prelude::*;

pub mod rpc;
pub mod rpc_async;
pub mod unit;
pub mod caller;
pub mod two_units;
pub mod two_units_async;

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip_restores_first_snapshot() {
        let (first_snapshot, _half) = crate::caller::round_trip().expect("round trip failed");
        // The device ran with value 3 before the first save.
        assert_eq!(first_snapshot, 3);
    }

    #[test]
    fn two_unit_restore_redrives_pin() {
        let (target_snapshot, source_snapshot, level) =
            crate::two_units::two_unit_round_trip().expect("round trip failed");
        assert_eq!(target_snapshot, 1);
        // The source counter was 5 when saved, so the restored pin is high,
        // even though the counter was 4 (pin low) just before the restore.
        assert_eq!(source_snapshot, 5);
        assert!(level);
    }

    #[test]
    fn async_two_unit_restore_redrives_pin() {
        let (target_snapshot, source_snapshot, level) =
            crate::two_units_async::run_two_unit_round_trip().expect("round trip failed");
        assert_eq!(target_snapshot, 1);
        assert_eq!(source_snapshot, 5);
        assert!(level);
    }
}
