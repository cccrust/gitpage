import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { listIssues, getRepo } from '../api'
import type { IssueWithAuthor, Repo } from '../api'
import Spinner from '../components/Spinner'

export default function IssueList() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [issues, setIssues] = useState<IssueWithAuthor[]>([])
  const [filter, setFilter] = useState('open')
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }
    setLoading(true)
    getRepo(numId)
      .then(r => { setRepo(r.repo); return listIssues(numId, filter || undefined) })
      .then(r => setIssues(r.issues))
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id, filter])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">倉庫不存在</div>

  const tabs = ['open', 'closed', 'all']

  return (
    <div className="page" style={{ padding: '16px 0 80px' }}>
      <div className="breadcrumb">
        <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <strong>Issues</strong>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 16 }}>
        <h2 style={{ fontSize: 20, fontWeight: 600 }}>Issues</h2>
        <Link to={`/repo/${repo.id}/issues/new`} className="btn-sm" style={{ fontWeight: 600 }}>New Issue</Link>
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

      {issues.length === 0 ? (
        <div className="empty-state"><p>No issues yet</p></div>
      ) : (
        <div>
          {issues.map(i => (
            <Link key={i.issue.id} to={`/repo/${repo.id}/issues/${i.issue.number}`}
              style={{ display: 'block', padding: '12px 0', borderBottom: '1px solid #e5e5e5', textDecoration: 'none' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontSize: 16 }}>{i.issue.state === 'open' ? '🟢' : '🔴'}</span>
                <strong style={{ fontSize: 15 }}>{i.issue.title}</strong>
              </div>
              <div style={{ fontSize: 13, color: '#7c7c7c', marginTop: 4 }}>
                #{i.issue.number} · {i.issue.state} · {i.author_username} · {new Date(i.issue.created_at).toLocaleDateString()}
                {i.labels.length > 0 && (
                  <span style={{ marginLeft: 8, display: 'inline-flex', gap: 4 }}>
                    {i.labels.map(l => (
                      <span key={l.id} style={{
                        display: 'inline-block', padding: '1px 6px', borderRadius: 3,
                        fontSize: 11, fontWeight: 600, background: `#${l.color}22`, color: `#${l.color}`
                      }}>{l.name}</span>
                    ))}
                  </span>
                )}
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
