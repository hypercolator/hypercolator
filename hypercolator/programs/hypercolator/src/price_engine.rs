/// Number of slots in the TWAP observation window (~30 minutes at ~2 slots/sec).
pub const TWAP_WINDOW_SLOTS: u64 = 3_600;

/// Minimum reserve size (in lamports) required in both legs of the AMM pool.
pub const MIN_LIQUIDITY: u64 = 1_000_000;

/// Maximum acceptable deviation between spot and TWAP, in basis points.
pub const MAX_DEVIATION_BPS: u64 = 2_000; // 20%

/// Compute the Q32 spot price from AMM x*y=k reserves.
///
/// price = (reserve_b << 32) / reserve_a  — units: quote per base, scaled 2^32.
///
/// Returns `None` if `reserve_a` is zero or the result overflows `u64`.
pub fn spot_price_q32(reserve_a: u64, reserve_b: u64) -> Option<u64> {
    if reserve_a == 0 {
        return None;
    }
    let price = ((reserve_b as u128) << 32) / reserve_a as u128;
    u64::try_from(price).ok()
}

/// Returns `true` when the spot price does not exceed the TWAP by more than
/// MAX_DEVIATION_BPS / 10_000 (20%).
///
/// Only upward deviations are rejected: a crash in spot price should still
/// allow liquidation (price is real, not manipulated).  A spike above TWAP
/// indicates a pump attack and must be rejected to protect traders.
///
/// Uses integer arithmetic: spot <= twap * (10_000 + MAX_DEVIATION_BPS) / 10_000
///   ⟺ spot * 10_000 <= twap * (10_000 + MAX_DEVIATION_BPS)
pub fn within_deviation(spot_q32: u64, twap_q32: u64) -> bool {
    if twap_q32 == 0 {
        return false;
    }
    // spot <= twap: no upward deviation at all
    if spot_q32 <= twap_q32 {
        return true;
    }
    let diff = spot_q32 - twap_q32;
    (diff as u128) * 10_000 <= (twap_q32 as u128) * (MAX_DEVIATION_BPS as u128)
}

/// Returns `true` when both AMM reserves meet the minimum liquidity floor.
pub fn sufficient_liquidity(reserve_a: u64, reserve_b: u64) -> bool {
    reserve_a >= MIN_LIQUIDITY && reserve_b >= MIN_LIQUIDITY
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- spot_price_q32 ---

    #[test]
    fn spot_price_one_to_one() {
        // 1:1 reserves -> price = 2^32
        assert_eq!(spot_price_q32(1_000_000, 1_000_000), Some(1u64 << 32));
    }

    #[test]
    fn spot_price_two_to_one() {
        // reserve_b = 2 * reserve_a  -> price = 2 * 2^32
        let p = spot_price_q32(1_000_000, 2_000_000).unwrap();
        assert_eq!(p, 2u64 << 32);
    }

    #[test]
    fn spot_price_zero_reserve_a() {
        assert_eq!(spot_price_q32(0, 1_000_000), None);
    }

    #[test]
    fn spot_price_zero_reserve_b() {
        assert_eq!(spot_price_q32(1_000_000, 0), Some(0));
    }

    // --- within_deviation ---

    #[test]
    fn deviation_zero_is_within() {
        let p = spot_price_q32(1_000_000, 1_000_000).unwrap();
        assert!(within_deviation(p, p));
    }

    #[test]
    fn deviation_exactly_20pct_is_within() {
        let twap: u64 = 1_000_000;
        let spot: u64 = 1_200_000; // exactly +20%
        assert!(within_deviation(spot, twap));
    }

    #[test]
    fn deviation_above_20pct_rejected() {
        let twap: u64 = 1_000_000;
        let spot: u64 = 1_200_001; // 20.0001% -- just over
        assert!(!within_deviation(spot, twap));
    }

    #[test]
    fn deviation_downward_always_allowed() {
        // Sharp downward move: liquidation should still be permitted.
        let twap: u64 = 1_000_000;
        assert!(within_deviation(0, twap));
        assert!(within_deviation(1, twap));
        assert!(within_deviation(twap - 1, twap));
    }

    #[test]
    fn deviation_zero_twap_rejected() {
        assert!(!within_deviation(100, 0));
    }

    // --- sufficient_liquidity ---

    #[test]
    fn liquidity_both_above_min() {
        assert!(sufficient_liquidity(MIN_LIQUIDITY, MIN_LIQUIDITY));
        assert!(sufficient_liquidity(MIN_LIQUIDITY * 10, MIN_LIQUIDITY * 5));
    }

    #[test]
    fn liquidity_reserve_a_below_min() {
        assert!(!sufficient_liquidity(MIN_LIQUIDITY - 1, MIN_LIQUIDITY));
    }

    #[test]
    fn liquidity_reserve_b_below_min() {
        assert!(!sufficient_liquidity(MIN_LIQUIDITY, MIN_LIQUIDITY - 1));
    }

    #[test]
    fn liquidity_both_zero() {
        assert!(!sufficient_liquidity(0, 0));
    }
}
