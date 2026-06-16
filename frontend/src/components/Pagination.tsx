interface Props {
  page: number
  totalPages: number
  onPageChange: (page: number) => void
}

export default function Pagination({ page, totalPages, onPageChange }: Props) {
  if (totalPages <= 1) return null
  return (
    <div className="pagination" style={{ display: 'flex', gap: 8, justifyContent: 'center', marginTop: 16 }}>
      <button disabled={page <= 1} onClick={() => onPageChange(page - 1)}>Prev</button>
      <span style={{ fontSize: 13, padding: '4px 0' }}>{page} / {totalPages}</span>
      <button disabled={page >= totalPages} onClick={() => onPageChange(page + 1)}>Next</button>
    </div>
  )
}