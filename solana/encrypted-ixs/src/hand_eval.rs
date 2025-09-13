/**
 * @description
 * This module implements data-independent poker hand evaluation for a 5-card hand.
 * The logic is designed to run within Arcium's Multi-Party Computation (MPC) environment,
 * which means it avoids data-dependent control flow (like `if`, `else`, or `match` on
 * secret values).
 *
 * The primary function, `evaluate_hand`, calculates a unique `u64` score for any given
 * 5-card hand. This score is structured such that a higher score represents a stronger hand,
 * allowing for simple numerical comparison to determine a winner.
 *
 * @key_features
 * - Data-Independent: All calculations use arithmetic and bitwise operations instead of
 *   conditional branches, making it suitable for secure MPC.
 * - Unique Scoring: Each possible hand maps to a distinct `u64` score.
 * - Kicker Handling: Correctly sorts and incorporates kickers into the score for all hand types.
 * - Special Cases: Properly handles edge cases like the A-2-3-4-5 "wheel" straight.
 *
 * @dependencies
 * - arcis_imports: Provides types and functions for writing Arcis confidential instructions.
 *
 * @notes
 * - A card is represented as a `u8` from 0 to 51.
 * - `rank = card / 4` (0=Two, ..., 12=Ace)
 * - `suit = card % 4`
 * - The final score is composed as: `(HandRank << 20) | (Kicker1 << 16) | ... | (Kicker5 << 0)`.
 */
use arcis_imports::*;

// --- Constants for Hand Ranks ---
// These values determine the base score for each hand type.
const HIGH_CARD_RANK: u64 = 0;
const ONE_PAIR_RANK: u64 = 1;
const TWO_PAIR_RANK: u64 = 2;
const THREE_OF_A_KIND_RANK: u64 = 3;
const STRAIGHT_RANK: u64 = 4;
const FLUSH_RANK: u64 = 5;
const FULL_HOUSE_RANK: u64 = 6;
const FOUR_OF_A_KIND_RANK: u64 = 7;
const STRAIGHT_FLUSH_RANK: u64 = 8;

// --- Constants for Card Ranks ---
// Ace is high (12), Two is low (0).
const RANK_ACE: u8 = 12;
const RANK_FIVE: u8 = 3;
const RANK_FOUR: u8 = 2;
const RANK_THREE: u8 = 1;
const RANK_TWO: u8 = 0;

/// The main evaluation function. It orchestrates the entire process of scoring a 5-card hand.
///
/// # Arguments
/// * `hand` - A fixed-size array of 5 `u8` values, where each value represents a card from 0-51.
///
/// # Returns
/// A `u64` score representing the hand's strength.
pub fn evaluate_hand(hand: [u8; 5]) -> u64 {
    // 1. Prepare card data: extract and sort ranks, get suits.
    let mut ranks = [0u8; 5];
    let mut suits = [0u8; 5];
    for i in 0..5 {
        ranks[i] = hand[i] / 4;
        suits[i] = hand[i] % 4;
    }
    // Sorting ranks in descending order simplifies many downstream calculations.
    // Arcis provides a data-independent sort for integer arrays.
    ranks.sort();
    ranks.reverse();

    // 2. Create a frequency map (histogram) of ranks.
    let mut rank_counts = [0u8; 13];
    for &rank in ranks.iter() {
        rank_counts[rank as usize] += 1;
    }

    // 3. Detect hand features (flush, straight) in a data-independent way.
    let is_flush = (suits[0] == suits[1])
        & (suits[0] == suits[2])
        & (suits[0] == suits[3])
        & (suits[0] == suits[4]);

    let is_straight_gapped = (ranks[0] - ranks[4] == 4) & (ranks[0] != ranks[1]) & (ranks[1] != ranks[2]) & (ranks[2] != ranks[3]) & (ranks[3] != ranks[4]);

    // Handle the A-2-3-4-5 "wheel" straight.
    let is_wheel = (ranks[0] == RANK_ACE)
        & (ranks[1] == RANK_FIVE)
        & (ranks[2] == RANK_FOUR)
        & (ranks[3] == RANK_THREE)
        & (ranks[4] == RANK_TWO);

    let is_straight = is_straight_gapped | is_wheel;
    let is_straight_flush = is_straight & is_flush;

    // 4. Analyze rank counts to identify pairs, trips, etc.
    let mut num_quads = 0;
    let mut num_trips = 0;
    let mut num_pairs = 0;
    for &count in rank_counts.iter() {
        num_quads += (count == 4) as u8;
        num_trips += (count == 3) as u8;
        num_pairs += (count == 2) as u8;
    }

    let is_four_of_a_kind = num_quads == 1;
    let is_full_house = (num_trips == 1) & (num_pairs == 1);
    let is_three_of_a_kind = (num_trips == 1) & (num_pairs == 0);
    let is_two_pair = num_pairs == 2;
    let is_one_pair = (num_pairs == 1) & (num_trips == 0);

    // 5. Determine the final hand rank using mutually exclusive conditions.
    // This chain of boolean logic ensures only the highest possible rank is selected.
    let hand_rank = (is_straight_flush as u64 * STRAIGHT_FLUSH_RANK)
        + ((!is_straight_flush & is_four_of_a_kind) as u64 * FOUR_OF_A_KIND_RANK)
        + ((!is_straight_flush & !is_four_of_a_kind & is_full_house) as u64 * FULL_HOUSE_RANK)
        + ((!is_straight_flush & !is_four_of_a_kind & !is_full_house & is_flush) as u64 * FLUSH_RANK)
        + ((!is_straight_flush & !is_four_of_a_kind & !is_full_house & !is_flush & is_straight) as u64 * STRAIGHT_RANK)
        + ((!is_straight & !is_flush & is_three_of_a_kind) as u64 * THREE_OF_A_KIND_RANK)
        + ((!is_straight & !is_flush & !is_three_of_a_kind & is_two_pair) as u64 * TWO_PAIR_RANK)
        + ((!is_straight & !is_flush & !is_three_of_a_kind & !is_two_pair & is_one_pair) as u64 * ONE_PAIR_RANK)
        + ((!is_straight & !is_flush & !is_one_pair & !is_two_pair & !is_three_of_a_kind & !is_full_house & !is_four_of_a_kind) as u64 * HIGH_CARD_RANK);


    // 6. Determine the kickers in the correct order.
    // We sort ranks first by their frequency (count), then by their value.
    // This universally orders kickers correctly for any hand type.
    // For example, in a full house KKKQQ, K (count 3) comes before Q (count 2).
    // In two pair AAKKQ, A (count 2) comes before K (count 2) because it's a higher rank.
    let mut packed_ranks = [0u16; 13];
    for i in 0..13 {
        // Pack count and rank into a u16 for sorting: (count << 8) | rank
        packed_ranks[i] = ((rank_counts[i] as u16) << 8) | (i as u16);
    }
    packed_ranks.sort();
    packed_ranks.reverse();

    let mut ordered_kickers = [0u8; 5];
    let mut kicker_idx = 0;
    for i in 0..13 {
        let count = (packed_ranks[i] >> 8) as u8;
        let rank = (packed_ranks[i] & 0xFF) as u8;
        // This loop is data-independent because its iteration count is fixed.
        for _ in 0..count {
            if kicker_idx < 5 {
                ordered_kickers[kicker_idx] = rank;
                kicker_idx += 1;
            }
        }
    }
    
    // Special case for the wheel straight (A-5-4-3-2), the '5' is the high card for rank, not the Ace.
    let wheel_kicker_override = [RANK_FIVE, RANK_FOUR, RANK_THREE, RANK_TWO, RANK_ACE];
    for i in 0..5 {
        // This is a multiplexer: `(cond * val_if_true) + (!cond * val_if_false)`
        ordered_kickers[i] = (is_wheel as u8 * wheel_kicker_override[i]) + ((!is_wheel) as u8 * ordered_kickers[i]);
    }

    // 7. Assemble the final score by bit-shifting the rank and kickers together.
    // Hand Rank (4 bits) | Kicker 1 (4 bits) | Kicker 2 (4 bits) | ... | Kicker 5 (4 bits)
    let mut score = hand_rank << 20;
    score |= (ordered_kickers[0] as u64) << 16;
    score |= (ordered_kickers[1] as u64) << 12;
    score |= (ordered_kickers[2] as u64) << 8;
    score |= (ordered_kickers[3] as u64) << 4;
    score |= (ordered_kickers[4] as u64) << 0;

    score
}