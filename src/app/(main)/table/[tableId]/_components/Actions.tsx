export function Actions() {
  return (
    <div className="flex gap-2 p-4 bg-gray-100 rounded-lg">
      <button className="px-4 py-2 bg-blue-500 text-white rounded-md">Check</button>
      <button className="px-4 py-2 bg-green-500 text-white rounded-md">Call</button>
      <button className="px-4 py-2 bg-red-500 text-white rounded-md">Fold</button>
      <button className="px-4 py-2 bg-purple-500 text-white rounded-md">Raise</button>
    </div>
  );
}