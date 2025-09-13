use arcis_imports::*;

pub mod determine_winner;
pub mod hand_eval;
pub mod reveal_community_cards;
pub mod shuffle_and_deal;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    #[derive(Clone, Copy)]
    pub struct Deck {
        pub cards: [u8; 48],
        pub dealt_community_cards: u8,
    }

    #[instruction]
    pub fn shuffle_and_deal(
        player1_pubkey: ArcisPublicKey,
        player2_pubkey: ArcisPublicKey,
    ) -> (Enc<Shared, [u8; 2]>, Enc<Shared, [u8; 2]>, Enc<Mxe, Deck>) {
        let mut deck: [u8; 52] = [0; 52];
        for i in 0..52 {
            deck[i] = i as u8;
        }

        ArcisRNG::shuffle(&mut deck);

        let p1_cards = [deck[0], deck[1]];
        let p2_cards = [deck[2], deck[3]];

        let mut board_deck_cards = [0u8; 48];
        for i in 0..48 {
            board_deck_cards[i] = deck[i + 4];
        }

        let board_deck = Deck {
            cards: board_deck_cards,
            dealt_community_cards: 0,
        };

        let player1_owner = Shared::new(player1_pubkey);
        let player2_owner = Shared::new(player2_pubkey);
        let mxe_owner = Mxe::get();

        let enc_p1_cards = player1_owner.from_arcis(p1_cards);
        let enc_p2_cards = player2_owner.from_arcis(p2_cards);
        let enc_board_deck = mxe_owner.from_arcis(board_deck);

        (enc_p1_cards, enc_p2_cards, enc_board_deck)
    }

    #[instruction]
    pub fn reveal_community_cards(
        deck_ctxt: Enc<Mxe, Deck>,
        phase: u8,
    ) -> (Enc<Mxe, Deck>, Enc<Mxe, [u8; 3]>) {
        let mut deck = deck_ctxt.to_arcis();

        let is_flop = phase == 0;
        let is_turn = phase == 1;
        let is_river = phase == 2;

        let mut revealed_cards = [255u8; 3];
        let start_idx = deck.dealt_community_cards as usize;

        if is_flop {
            revealed_cards[0] = deck.cards[start_idx];
            revealed_cards[1] = deck.cards[start_idx + 1];
            revealed_cards[2] = deck.cards[start_idx + 2];
            deck.dealt_community_cards += 3;
        }
        if is_turn {
            revealed_cards[0] = deck.cards[start_idx];
            deck.dealt_community_cards += 1;
        }
        if is_river {
            revealed_cards[0] = deck.cards[start_idx];
            deck.dealt_community_cards += 1;
        }

        let mxe_owner1 = Mxe::get();
        let mxe_owner2 = Mxe::get();
        let enc_deck = mxe_owner1.from_arcis(deck);
        let enc_revealed_cards = mxe_owner2.from_arcis(revealed_cards);

        (enc_deck, enc_revealed_cards)
    }

    /// Determines the winner of a poker hand at showdown.
    ///
    /// This instruction takes the encrypted hole cards for two players and the public community
    /// cards, confidentially evaluates each player's best 5-card hand, and returns the index
    /// of the winning player without revealing the losing hand.
    ///
    /// # Arguments
    /// * `p1_cards_ctxt` - Player 1's two hole cards, encrypted with a shared key.
    /// * `p2_cards_ctxt` - Player 2's two hole cards, encrypted with a shared key.
    /// * `board` - The five public community cards (unencrypted).
    ///
    /// # Returns
    /// A `u8` indicating the winner:
    /// - `0`: Player 1 wins.
    /// - `1`: Player 2 wins.
    /// - `2`: It's a tie (split pot).
    #[instruction]
    pub fn determine_winner(
        p1_cards_ctxt: Enc<Shared, [u8; 2]>,
        p2_cards_ctxt: Enc<Shared, [u8; 2]>,
        board: [u8; 5],
    ) -> u8 {
        // Define the hand evaluation functions directly here since we can't import them
        // due to Arcis restrictions
        
        // --- Constants for Hand Ranks ---
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
        const RANK_ACE: u8 = 12;
        const RANK_FIVE: u8 = 3;
        const RANK_FOUR: u8 = 2;
        const RANK_THREE: u8 = 1;
        const RANK_TWO: u8 = 0;

        // The main evaluation function for a 5-card hand
        fn evaluate_hand(hand: [u8; 5]) -> u64 {
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
                // Since bit shifting is not supported, we use multiplication:
                packed_ranks[i] = ((rank_counts[i] as u16) * 256) + (i as u16);
            }
            packed_ranks.sort();
            packed_ranks.reverse();

            let mut ordered_kickers = [0u8; 5];
            let mut kicker_idx = 0u8;
            for i in 0..13 {
                let count = (packed_ranks[i] / 256) as u8;
                let rank = (packed_ranks[i] % 256) as u8;
                
                // Unroll the loop since count can vary between 0 and 5
                // Use arithmetic multiplexers to conditionally add kickers
                let should_add_0 = (count > 0) & (kicker_idx < 5);
                ordered_kickers[kicker_idx as usize] = (should_add_0 as u8 * rank) + ((!should_add_0) as u8 * ordered_kickers[kicker_idx as usize]);
                kicker_idx += should_add_0 as u8;
                
                let should_add_1 = (count > 1) & (kicker_idx < 5);
                ordered_kickers[kicker_idx as usize] = (should_add_1 as u8 * rank) + ((!should_add_1) as u8 * ordered_kickers[kicker_idx as usize]);
                kicker_idx += should_add_1 as u8;
                
                let should_add_2 = (count > 2) & (kicker_idx < 5);
                ordered_kickers[kicker_idx as usize] = (should_add_2 as u8 * rank) + ((!should_add_2) as u8 * ordered_kickers[kicker_idx as usize]);
                kicker_idx += should_add_2 as u8;
                
                let should_add_3 = (count > 3) & (kicker_idx < 5);
                ordered_kickers[kicker_idx as usize] = (should_add_3 as u8 * rank) + ((!should_add_3) as u8 * ordered_kickers[kicker_idx as usize]);
                kicker_idx += should_add_3 as u8;
                
                let should_add_4 = (count > 4) & (kicker_idx < 5);
                ordered_kickers[kicker_idx as usize] = (should_add_4 as u8 * rank) + ((!should_add_4) as u8 * ordered_kickers[kicker_idx as usize]);
                kicker_idx += should_add_4 as u8;
            }
            
            // Special case for the wheel straight (A-5-4-3-2), the '5' is the high card for rank, not the Ace.
            let wheel_kicker_override = [RANK_FIVE, RANK_FOUR, RANK_THREE, RANK_TWO, RANK_ACE];
            for i in 0..5 {
                // This is a multiplexer: `(cond * val_if_true) + (!cond * val_if_false)`
                ordered_kickers[i] = (is_wheel as u8 * wheel_kicker_override[i]) + ((!is_wheel) as u8 * ordered_kickers[i]);
            }

            // 7. Assemble the final score by bit-shifting the rank and kickers together.
            // Hand Rank (4 bits) | Kicker 1 (4 bits) | Kicker 2 (4 bits) | ... | Kicker 5 (4 bits)
            // Since bit shifting is not supported, we use multiplication:
            let mut score = hand_rank * 1048576; // 2^20
            score = score + (ordered_kickers[0] as u64) * 65536; // 2^16
            score = score + (ordered_kickers[1] as u64) * 4096; // 2^12
            score = score + (ordered_kickers[2] as u64) * 256; // 2^8
            score = score + (ordered_kickers[3] as u64) * 16; // 2^4
            score = score + (ordered_kickers[4] as u64) * 1; // 2^0

            score
        }

        // Finds the highest possible score from a 7-card hand
        fn find_best_hand_from_seven(seven_cards: [u8; 7]) -> u64 {
            // All 21 combinations of 5-card hands from 7 cards, represented by indices.
            const COMBINATIONS: [[usize; 5]; 21] = [
                [0,1,2,3,4], [0,1,2,3,5], [0,1,2,3,6], [0,1,2,4,5], [0,1,2,4,6],
                [0,1,2,5,6], [0,1,3,4,5], [0,1,3,4,6], [0,1,3,5,6], [0,1,4,5,6],
                [0,2,3,4,5], [0,2,3,4,6], [0,2,3,5,6], [0,2,4,5,6], [0,3,4,5,6],
                [1,2,3,4,5], [1,2,3,4,6], [1,2,3,5,6], [1,2,4,5,6], [1,3,4,5,6],
                [2,3,4,5,6]
            ];

            let mut max_score = 0u64;

            // Iterate through all combinations, evaluate each 5-card hand, and keep track of the max score.
            // This loop is data-independent as it always runs 21 times.
            for combo in COMBINATIONS {
                let mut current_hand = [0u8; 5];
                current_hand[0] = seven_cards[combo[0]];
                current_hand[1] = seven_cards[combo[1]];
                current_hand[2] = seven_cards[combo[2]];
                current_hand[3] = seven_cards[combo[3]];
                current_hand[4] = seven_cards[combo[4]];
                
                let score = evaluate_hand(current_hand);
                
                // Data-independent update of max_score using an arithmetic multiplexer.
                // This is equivalent to `if score > max_score { max_score = score; }`
                // but avoids data-dependent branching.
                let is_greater = score > max_score;
                max_score = (is_greater as u64 * score) + ((!is_greater) as u64 * max_score);
            }

            max_score
        }
        
        let p1_cards = p1_cards_ctxt.to_arcis();
        let p2_cards = p2_cards_ctxt.to_arcis();

        // Combine hole cards and board for player 1
        let p1_seven_cards = [
            p1_cards[0],
            p1_cards[1],
            board[0],
            board[1],
            board[2],
            board[3],
            board[4],
        ];

        // Combine hole cards and board for player 2
        let p2_seven_cards = [
            p2_cards[0],
            p2_cards[1],
            board[0],
            board[1],
            board[2],
            board[3],
            board[4],
        ];

        // Evaluate the best 5-card hand for each player using the helper function.
        let p1_score = find_best_hand_from_seven(p1_seven_cards);
        let p2_score = find_best_hand_from_seven(p2_seven_cards);

        // Data-independent comparison to determine the winner index.
        let p1_wins = p1_score > p2_score;
        let p2_wins = p2_score > p1_score;

        // This multiplexer logic selects the correct winner index without branching.
        // If p1_wins is true (1), the first term is 0.
        // If p2_wins is true (1), the second term is 1.
        // If neither is true (tie), the third term is 2.
        let winner_index =
            (p1_wins as u8 * 0) + (p2_wins as u8 * 1) + ((!p1_wins & !p2_wins) as u8 * 2);

        winner_index.reveal()
    }
}