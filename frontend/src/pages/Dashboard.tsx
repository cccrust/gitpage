import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { listRepos, type Repo, isLoggedIn } from '../api'

export default function Dashboard() {
  const [repos, setRepos] = useState<Repo[]>([])
  const [searchQ, setSearchQ] = useState('')
  const [searchResults, setSearchResults] = useState<Repo[] | null>(null)
  const [searching, setSearching] = useState(false)
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const loggedIn = isLoggedIn()

  useEffect(() => {
    if (!loggedIn) {
      setLoading(false)
      return
    }
    listRepos()
      .then(r => setRepos(r.repos))
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [loggedIn])

  const doSearch = async () => {
    if (!searchQ.trim()) { setSearchResults(null); return }
    setSearching(true)
    try {
      const r = await fetch(`/api/repos/search?q=${encodeURIComponent(searchQ)}`)
      const d = await r.json()
      setSearchResults(d.repos)
    } catch { setSearchResults([]) }
    setSearching(false)
  }

  const displayRepos = searchResults !== null ? searchResults : repos

  if (!loggedIn) {
    return (
      <div className="dashboard">
        <div className="empty-state">
          <h1 style={{ fontSize: 24, marginBottom: 8 }}>gitpage</h1>
          <p>Self-hosted Git platform</p>
          <p style={{ marginTop: 16 }}>
            <Link to="/login" style={{ textDecoration: 'underline' }}>Login</Link>
            {' or '}
            <Link to="/register" style={{ textDecoration: 'underline' }}>Register</Link>
          </p>
        </div>
      </div>
    )
  }

  if (loading) return <div className="loading">Loading...</div>
  if (err) return <div className="error-box">{err}</div>

  return (
    <div className="dashboard">
      <div className="head">
        <h1>Repositories</h1>
        <Link to="/new" className="btn-sm">+ New</Link>
      </div>

      <div className="search-bar" style={{ marginBottom: 12 }}>
        <input
          type="text"
          placeholder="Search public repos..."
          value={searchQ}
          onChange={e => setSearchQ(e.target.value)}
          onKeyUp={e => e.key === 'Enter' && doSearch()}
          style={{ fontSize: 14, padding: '8px 10px', width: '100%', background: '#111', border: '1px solid #333', borderRadius: 6, color: '#e0e0e0' }}
        />
      </div>

      {searching && <div className="loading" style={{ padding: 8 }}>Searching...</div>}

      {displayRepos.length === 0 ? (
        <div className="empty-state">
          <p>{searchResults !== null ? 'No results' : 'No repositories yet'}</p>
          {searchResults === null && <Link to="/new" style={{ fontSize: 14, textDecoration: 'underline' }}>Create one</Link>}
        </div>
      ) : (
        displayRepos.map(r => (
          <Link key={r.id} to={`/repo/${r.id}`} className="repo-card">
            <div className="name">{r.name}</div>
            {r.description && <div className="desc">{r.description}</div>}
            <div className="meta">
              <span>{r.is_private ? 'Private' : 'Public'}</span>
              <span>{r.updated_at?.slice(0, 10)}</span>
            </div>
          </Link>
        ))
      )}
    </div>
  )
}
