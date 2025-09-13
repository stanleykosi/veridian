export default function TablePage({ params }: { params: { tableId: string } }) {
  return (
    <div>
      <h1>Table {params.tableId}</h1>
      <p>Poker table view</p>
    </div>
  );
}