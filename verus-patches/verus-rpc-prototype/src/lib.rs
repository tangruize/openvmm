//! Stage 1 prototype: passing fractional ghost ownership through an RPC.

#[allow(unused_imports)]
use vstd::prelude::*;

pub mod rpc_t;
pub mod rpc_async_t;
pub mod mesh_rpc_t;
pub mod mesh_unit;
pub mod mutex_t;
pub mod mutex_unit;
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

    #[test]
    fn mesh_round_trip_restores_saved_state() {
        let (saved, _half) =
            crate::rpc_async_t::block_on(crate::mesh_unit::round_trip()).expect("round trip failed");
        assert_eq!(saved, 3);
    }

    #[test]
    fn mutex_registry_collects_halves() {
        let mut units = crate::mutex_unit::Units::new();
        units.add(1, vstd::prelude::Tracked::assume_new());
        units.add(2, vstd::prelude::Tracked::assume_new());
        units.collect();
        units.add(3, vstd::prelude::Tracked::assume_new());
        units.collect();
    }

    #[test]
    fn mutex_unverified_lock_sees_released_value() {
        use crate::mutex_unit::{Registry, RegistryPred};
        use vstd::prelude::{Ghost, Tracked};
        let r = Registry { ids: Vec::new(), inbox: Tracked::assume_new() };
        let m = crate::mutex_t::Mutex::<Registry, RegistryPred>::new(r, Ghost::assume_new());
        let (mut r, handle) = m.acquire();
        r.ids.push((1, Ghost::assume_new()));
        handle.release(r);
        m.lock().ids.push((2, Ghost::assume_new()));
        let (r, handle) = m.acquire();
        assert_eq!(r.ids.iter().map(|(n, _)| *n).collect::<Vec<_>>(), [1, 2]);
        handle.release(r);
    }
}
