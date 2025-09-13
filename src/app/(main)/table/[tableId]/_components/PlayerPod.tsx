export function PlayerPod({ name, chips }: { name: string; chips: number }) {
  return (
    <div className="w-48 h-32 bg-gray-200 rounded-lg p-3 flex flex-col">
      <div className="font-bold">{name}</div>
      <div className="mt-2">${chips}</div>
      <div className="mt-2 flex gap-1">
        <div className="w-8 h-12 bg-white rounded border"></div>
        <div className="w-8 h-12 bg-white rounded border"></div>
      </div>
    </div>
  );
}