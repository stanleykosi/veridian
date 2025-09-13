export function PokerChip({ value }: { value: number }) {
  return (
    <div className="w-12 h-12 rounded-full bg-gradient-to-br from-yellow-400 to-yellow-600 border-2 border-yellow-700 flex items-center justify-center shadow-md">
      <span className="text-white font-bold text-sm">${value}</span>
    </div>
  );
}