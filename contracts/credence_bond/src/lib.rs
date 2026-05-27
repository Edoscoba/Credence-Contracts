#![no_std]

use credence_errors::ContractError;

// ---------------------------------------------------------------------------
// Bond creation result
// ---------------------------------------------------------------------------

/// Represents a validated, created bond.
///
/// All fields are guaranteed to satisfy the invariants enforced by
/// [`create_bond`]:
/// - `amount > 0`
/// - `duration > 0`
/// - If `is_rolling`, then `0 < notice_period_duration <= duration`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bond {
    /// Bonded amount (strictly positive).
    pub amount: i128,
    /// Bond start timestamp (ledger seconds).
    pub bond_start: u64,
    /// Duration of the bond in seconds (strictly positive).
    pub duration: u64,
    /// Whether this is a rolling bond.
    pub is_rolling: bool,
    /// Notice period for rolling-bond withdrawals.
    /// Meaningful only when `is_rolling` is `true`.
    pub notice_period_duration: u64,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Returns `true` when `amount` is a valid bond amount (strictly positive).
///
/// This is the single source of truth for the amount rule; [`create_bond`]
/// delegates to this function so the rule is never duplicated.
///
/// # Examples
/// ```
/// use credence_bond::is_valid_bond;
/// assert!(is_valid_bond(1));
/// assert!(!is_valid_bond(0));
/// assert!(!is_valid_bond(-1));
/// ```
pub fn is_valid_bond(amount: i128) -> bool {
    amount > 0
}

// ---------------------------------------------------------------------------
// create_bond
// ---------------------------------------------------------------------------

/// Creates and returns a validated [`Bond`].
///
/// # Parameters
/// - `amount` — bonded amount in the smallest token unit.
/// - `bond_start` — ledger timestamp at which the bond begins.
/// - `duration` — length of the bond in seconds.
/// - `is_rolling` — whether the bond auto-renews and requires a notice period
///   before withdrawal.
/// - `notice_period_duration` — advance notice required before a rolling-bond
///   withdrawal can be executed. Ignored when `is_rolling` is `false`.
///
/// # Preconditions
/// | Condition | Error |
/// |-----------|-------|
/// | `amount > 0` | [`ContractError::InvalidBondAmount`] |
/// | `duration > 0` | [`ContractError::InvalidBondDuration`] |
/// | `is_rolling` → `notice_period_duration > 0` | [`ContractError::InvalidNoticePeriod`] |
/// | `is_rolling` → `notice_period_duration <= duration` | [`ContractError::InvalidNoticePeriod`] |
/// | `bond_start + duration` does not overflow `u64` | [`ContractError::Overflow`] |
///
/// # Errors
/// Returns a typed [`ContractError`] for every invalid input combination.
/// No panics are used.
///
/// # Examples
/// ```
/// use credence_bond::create_bond;
/// // Valid non-rolling bond
/// let bond = create_bond(100, 0, 3600, false, 0).unwrap();
/// assert_eq!(bond.amount, 100);
///
/// // Valid rolling bond
/// let bond = create_bond(50, 0, 7200, true, 3600).unwrap();
/// assert_eq!(bond.notice_period_duration, 3600);
/// ```
pub fn create_bond(
    amount: i128,
    bond_start: u64,
    duration: u64,
    is_rolling: bool,
    notice_period_duration: u64,
) -> Result<Bond, ContractError> {
    // 1. Amount must be strictly positive — delegate to is_valid_bond so the
    //    rule lives in exactly one place.
    if !is_valid_bond(amount) {
        return Err(ContractError::InvalidBondAmount);
    }

    // 2. Duration must be strictly positive.
    if duration == 0 {
        return Err(ContractError::InvalidBondDuration);
    }

    // 3. Rolling-bond notice-period semantics.
    if is_rolling {
        if notice_period_duration == 0 {
            return Err(ContractError::InvalidNoticePeriod);
        }
        if notice_period_duration > duration {
            return Err(ContractError::InvalidNoticePeriod);
        }
    }

    // 4. Overflow guard on the bond end timestamp.
    bond_start
        .checked_add(duration)
        .ok_or(ContractError::Overflow)?;

    Ok(Bond {
        amount,
        bond_start,
        duration,
        is_rolling,
        notice_period_duration,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod fuzz;

#[cfg(test)]
mod tests {
    use super::*;
    use credence_errors::ContractError;

    // -----------------------------------------------------------------------
    // is_valid_bond
    // -----------------------------------------------------------------------

    #[test]
    fn is_valid_bond_positive_amount() {
        assert!(is_valid_bond(1));
        assert!(is_valid_bond(1_000_000));
        assert!(is_valid_bond(i128::MAX));
    }

    #[test]
    fn is_valid_bond_zero_is_invalid() {
        assert!(!is_valid_bond(0));
    }

    #[test]
    fn is_valid_bond_negative_is_invalid() {
        assert!(!is_valid_bond(-1));
        assert!(!is_valid_bond(-5));
        assert!(!is_valid_bond(i128::MIN));
    }

    // -----------------------------------------------------------------------
    // create_bond — invalid amount
    // -----------------------------------------------------------------------

    #[test]
    fn create_bond_rejects_zero_amount() {
        let err = create_bond(0, 0, 3600, false, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondAmount);
    }

    #[test]
    fn create_bond_rejects_negative_amount() {
        let err = create_bond(-1, 0, 3600, false, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondAmount);
    }

    #[test]
    fn create_bond_rejects_large_negative_amount() {
        let err = create_bond(i128::MIN, 0, 3600, false, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondAmount);
    }

    // -----------------------------------------------------------------------
    // create_bond — invalid duration
    // -----------------------------------------------------------------------

    #[test]
    fn create_bond_rejects_zero_duration() {
        let err = create_bond(100, 0, 0, false, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondDuration);
    }

    #[test]
    fn create_bond_rejects_zero_duration_rolling() {
        // duration=0 should fail on InvalidBondDuration before reaching notice checks
        let err = create_bond(100, 0, 0, true, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondDuration);
    }

    // -----------------------------------------------------------------------
    // create_bond — invalid notice period (rolling bonds)
    // -----------------------------------------------------------------------

    #[test]
    fn create_bond_rejects_zero_notice_for_rolling_bond() {
        let err = create_bond(100, 0, 3600, true, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidNoticePeriod);
    }

    #[test]
    fn create_bond_rejects_notice_greater_than_duration() {
        let err = create_bond(100, 0, 3600, true, 3601).unwrap_err();
        assert_eq!(err, ContractError::InvalidNoticePeriod);
    }

    #[test]
    fn create_bond_rejects_notice_much_greater_than_duration() {
        let err = create_bond(100, 0, 100, true, u64::MAX).unwrap_err();
        assert_eq!(err, ContractError::InvalidNoticePeriod);
    }

    // -----------------------------------------------------------------------
    // create_bond — overflow guard
    // -----------------------------------------------------------------------

    #[test]
    fn create_bond_rejects_overflow_on_bond_end() {
        let err = create_bond(100, u64::MAX, 1, false, 0).unwrap_err();
        assert_eq!(err, ContractError::Overflow);
    }

    #[test]
    fn create_bond_rejects_overflow_both_max() {
        let err = create_bond(100, u64::MAX, u64::MAX, false, 0).unwrap_err();
        assert_eq!(err, ContractError::Overflow);
    }

    // -----------------------------------------------------------------------
    // create_bond — valid paths (regression prevention)
    // -----------------------------------------------------------------------

    #[test]
    fn create_bond_valid_non_rolling() {
        let bond = create_bond(100, 1000, 3600, false, 0).unwrap();
        assert_eq!(bond.amount, 100);
        assert_eq!(bond.bond_start, 1000);
        assert_eq!(bond.duration, 3600);
        assert!(!bond.is_rolling);
        assert_eq!(bond.notice_period_duration, 0);
    }

    #[test]
    fn create_bond_valid_rolling_notice_less_than_duration() {
        let bond = create_bond(50, 0, 7200, true, 3600).unwrap();
        assert!(bond.is_rolling);
        assert_eq!(bond.notice_period_duration, 3600);
    }

    #[test]
    fn create_bond_valid_rolling_notice_equals_duration() {
        // Boundary: notice == duration is explicitly allowed
        let bond = create_bond(50, 0, 3600, true, 3600).unwrap();
        assert!(bond.is_rolling);
        assert_eq!(bond.notice_period_duration, 3600);
    }

    #[test]
    fn create_bond_valid_max_amount() {
        // i128::MAX is a valid positive amount
        let bond = create_bond(i128::MAX, 0, 1, false, 0).unwrap();
        assert_eq!(bond.amount, i128::MAX);
    }

    #[test]
    fn create_bond_valid_minimum_positive_amount() {
        let bond = create_bond(1, 0, 1, false, 0).unwrap();
        assert_eq!(bond.amount, 1);
    }

    #[test]
    fn create_bond_valid_minimum_duration() {
        // duration=1 is the smallest valid duration
        let bond = create_bond(100, 0, 1, false, 0).unwrap();
        assert_eq!(bond.duration, 1);
    }

    #[test]
    fn create_bond_valid_rolling_minimum_notice() {
        // notice=1 with duration=1 is valid (notice == duration boundary)
        let bond = create_bond(100, 0, 1, true, 1).unwrap();
        assert_eq!(bond.notice_period_duration, 1);
    }

    #[test]
    fn create_bond_non_rolling_ignores_notice_period() {
        // For non-rolling bonds, notice_period_duration is not validated
        let bond = create_bond(100, 0, 3600, false, 9999).unwrap();
        assert!(!bond.is_rolling);
        assert_eq!(bond.notice_period_duration, 9999);
    }

    #[test]
    fn create_bond_valid_no_overflow_at_boundary() {
        // bond_start=0, duration=u64::MAX — no overflow
        let bond = create_bond(100, 0, u64::MAX, false, 0).unwrap();
        assert_eq!(bond.duration, u64::MAX);
    }

    // -----------------------------------------------------------------------
    // Error ordering: amount checked before duration
    // -----------------------------------------------------------------------

    #[test]
    fn create_bond_amount_checked_before_duration() {
        // Both amount and duration are invalid; amount error should surface first
        let err = create_bond(0, 0, 0, false, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondAmount);
    }

    #[test]
    fn create_bond_duration_checked_before_notice() {
        // Both duration and notice are invalid; duration error should surface first
        let err = create_bond(100, 0, 0, true, 0).unwrap_err();
        assert_eq!(err, ContractError::InvalidBondDuration);
    }
}
