import { PlayerPod } from "./PlayerPod";
import { CommunityCards } from "./CommunityCards";
import { Actions } from "./Actions";

export function Table() {
  return (
    <div className="flex flex-col items-center gap-8 p-8">
      <CommunityCards />
      <div className="flex gap-4">
        <PlayerPod name="Player 1" chips={1000} />
        <PlayerPod name="Player 2" chips={1500} />
      </div>
      <Actions />
    </div>
  );
}