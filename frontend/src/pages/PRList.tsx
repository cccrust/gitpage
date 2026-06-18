import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { listPRs, getRepo } from '../api'
import type { PullRequestWithAuthor, Repo } from '../api'
import Spinner from '../components/Spinner'

export default function PRList() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [prs, setPrs] = useState<PullRequestWithAuthor[]>([])
  const [filter, setFilter] = useState('open')
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }
    setLoading(true)
    getRepo(numId)
      .then(r => { setRepo(r.repo); return listPRs(numId, filter || undefined) })
      .then(r => setPrs(r.prs))
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id, filter])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">倉庫不存在</div>

  const tabs = ['open', 'closed', 'merged', 'all']

  return (
    <div className="page" style={{ padding: '16px 0 80px' }}>
      <div className="breadcrumb">
        <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <strong>Pull Requests</strong>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 16 }}>
        <h2 style={{ fontSize: 20, fontWeight: 600 }}>Pull Requests</h2>
        <Link to={`/repo/${repo.id}/pulls/new`} className="btn-sm" style={{ fontWeight: 600 }}>New PR</Link>
      </div>

      <div style={{ display: 'flex', gap: 0, marginTop: 16, borderBottom: '1px solid #e5e5e5' }}>
        {tabs.map(t => (
          <button
            key={t}
            onClick={() => setFilter(t)}
            style={{
              padding: '8px 16px', border: 'none', background: 'none',
              fontWeight: filter === t ? 600 : 400,
              borderBottom: filter === t ? '2px solid #000' : '2px solid transparent',
              cursor: 'pointer', fontSize: 14, color: filter === t ? '#000' : '#7c7c7c'
            }}
          >
            {t === 'all' ? 'All' : t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {prs.length === 0 ? (
        <div className="empty-state"><p>No pull requests yet</p></div>
      ) : (
        <div>
          {prs.map(p => (
            <Link key={p.pr.id} to={`/repo/${repo.id}/pulls/${p.pr.number}`}
              style={{ display: 'block', padding: '12px 0', borderBottom: '1px solid #e5e5e5', textDecoration: 'none' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontSize: 16 }}>
                  {p.pr.state === 'open' ? '🟢' : p.pr.state === 'merged' ? '🟣' : '🔴'}
                </span>
                <strong style={{ fontSize: 15 }}>{p.pr.title}</strong>
              </div>
              <div style={{ fontSize: 13, color: '#7c7c7c', marginTop: 4 }}>
                #{p.pr.number} · {p.pr.state} · {p.author_username} · {new Date(p.pr.created_at).toLocaleDateString()}
                · {p.head_repo_owner}/{p.head_repo_name}:{p.pr.head_ref} → {p.pr.base_ref}
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
