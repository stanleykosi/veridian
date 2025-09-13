export interface Player {
  id: string;
  name: string;
  chips: number;
  cards: Card[];
  isDealer: boolean;
  isSmallBlind: boolean;
  isBigBlind: boolean;
}

export interface Card {
  suit: "hearts" | "diamonds" | "clubs" | "spades";
  rank: "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" | "J" | "Q" | "K" | "A";
}

export interface GameState {
  players: Player[];
  communityCards: Card[];
  pot: number;
  currentPlayer: string | null;
  round: "pre-flop" | "flop" | "turn" | "river" | "showdown";
}