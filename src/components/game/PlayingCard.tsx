export function PlayingCard({ suit, rank }: { suit: string; rank: string }) {
  return (
    <div className="w-20 h-28 bg-white rounded-md border border-gray-300 flex items-center justify-center">
      <div className="text-center">
        <div className="text-lg font-bold">{rank}</div>
        <div className="text-2xl">{suit}</div>
      </div>
    </div>
  );
}