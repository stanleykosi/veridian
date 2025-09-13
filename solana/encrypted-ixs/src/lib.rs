use arcis_imports::*;

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
}