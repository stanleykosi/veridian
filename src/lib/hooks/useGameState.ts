import { useState, useEffect } from "react";

export function useGameState() {
  const [gameState, setGameState] = useState<any>(null);

  useEffect(() => {
    // Placeholder for game state logic
    setGameState({
      players: [],
      communityCards: [],
      pot: 0,
    });
  }, []);

  return gameState;
}