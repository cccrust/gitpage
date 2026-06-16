export default function Spinner({ text = 'Loading...' }: { text?: string }) {
  return <div className="loading">{text}</div>
}