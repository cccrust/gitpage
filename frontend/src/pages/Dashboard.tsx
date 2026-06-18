import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { listRepos, type Repo, isLoggedIn } from '../api'
import Spinner from '../components/Spinner'

export default function Dashboard() {
  const [repos, setRepos] = useState<Repo[]>([])
  const [searchQ, setSearchQ] = useState('')
  const [searchResults, setSearchResults] = useState<Repo[] | null>(null)
  const [searching, setSearching] = useState(false)
  const [searchPage, setSearchPage] = useState(1)
  const [_searchTotal, setSearchTotal] = useState(0)
  const [searchTotalPages, setSearchTotalPages] = useState(0)
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

  const doSearch = async (page = 1) => {
    if (!searchQ.trim()) { setSearchResults(null); return }
    setSearching(true)
    setSearchPage(page)
    try {
      const r = await fetch(`/api/repos/search?q=${encodeURIComponent(searchQ)}&page=${page}&page_size=10`)
      const d = await r.json()
      setSearchResults(d.repos)
      setSearchTotal(d.total)
      setSearchTotalPages(d.total_pages)
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

  if (loading) return <Spinner />
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
          onChange={e => { setSearchQ(e.target.value); if (!e.target.value) setSearchResults(null) }}
          onKeyUp={e => e.key === 'Enter' && doSearch()}
          style={{ fontSize: 14, padding: '8px 10px', width: '100%', background: '#111', border: '1px solid #333', borderRadius: 6, color: '#e0e0e0' }}
        />
      </div>

      {searching && <Spinner text="Searching..." />}

      {displayRepos.length === 0 ? (
        <div className="empty-state">
          <p>{searchResults !== null ? 'No results' : 'No repositories yet'}</p>
          {searchResults === null && <Link to="/new" style={{ fontSize: 14, textDecoration: 'underline' }}>Create one</Link>}
        </div>
      ) : (
        <>
          {displayRepos.map(r => (
            <Link key={r.id} to={`/repo/${r.id}`} className="repo-card">
              <div className="name">{r.username ? `${r.username}/` : ''}{r.name}</div>
              {r.description && <div className="desc">{r.description}</div>}
              <div className="meta">
                <span>{r.is_private ? 'Private' : 'Public'}</span>
                <span>{r.updated_at?.slice(0, 10)}</span>
              </div>
            </Link>
          ))}
          {searchResults !== null && searchTotalPages > 1 && (
            <div className="pagination" style={{ display: 'flex', gap: 8, justifyContent: 'center', marginTop: 16 }}>
              <button disabled={searchPage <= 1} onClick={() => doSearch(searchPage - 1)}>Prev</button>
              <span style={{ fontSize: 13, padding: '4px 0' }}>{searchPage} / {searchTotalPages}</span>
              <button disabled={searchPage >= searchTotalPages} onClick={() => doSearch(searchPage + 1)}>Next</button>
            </div>
          )}
        </>
      )}
    </div>
  )
}