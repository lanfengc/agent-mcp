use super::InteractionLease;
use crate::Deadline;

#[test]
fn a_guarded_lease_carries_its_deadline_and_no_contention() {
    let deadline = Deadline::after(100).unwrap();
    let lease = InteractionLease::guarded(deadline, ()).unwrap();

    assert_eq!(lease.deadline(), deadline);
    assert_eq!(lease.contention_count(), 0);
}

#[test]
fn renewing_a_lease_replaces_only_the_deadline() {
    let lease = InteractionLease::guarded(Deadline::after(100).unwrap(), ()).unwrap();
    let renewed = lease.with_deadline(Deadline::after(200).unwrap());

    assert_ne!(renewed.deadline(), lease.deadline());
    assert_eq!(renewed.contention_count(), lease.contention_count());
}

#[test]
fn a_platform_lease_reports_the_contention_it_earned() {
    let deadline = Deadline::after(100).unwrap();
    let lease = InteractionLease::guarded_with_contention(deadline, (), 7);

    assert_eq!(lease.contention_count(), 7);
    assert_eq!(lease.with_deadline(deadline).contention_count(), 7);
}
