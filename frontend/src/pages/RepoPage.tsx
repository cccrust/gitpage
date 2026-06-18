import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, listTree, getReadme, listCommits, type Repo, type TreeEntry, type CommitInfo } from '../api'
import MarkdownView from '../components/MarkdownView'
import Spinner from '../components/Spinner'

export default function RepoPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [entries, setEntries] = useState<TreeEntry[]>([])
  const [readmeHtml, setReadmeHtml] = useState('')
  const [commits, setCommits] = useState<CommitInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [username, setUsername] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }

    setLoading(true)
    getRepo(numId)
      .then(async r => {
        setRepo(r.repo)
        const uname = r.org_name || r.username
        setUsername(uname)

        const [treeRes, readmeRes, commitRes] = await Promise.all([
          listTree(uname, r.repo.name, r.repo.default_branch),
          getReadme(uname, r.repo.name, r.repo.default_branch),
          listCommits(uname, r.repo.name, r.repo.default_branch),
        ])
        setEntries(treeRes.entries)
        if (readmeRes.has_readme && readmeRes.rendered) setReadmeHtml(readmeRes.rendered)
        setCommits(commitRes.commits)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">倉庫不存在</div>

  const branch = repo.default_branch

  return (
    <div className="repo-page">
      <div className="repo-header">
        <div className="breadcrumb">
          <Link to="/">~</Link> / <Link to={`/u/${username}`} style={{ color: '#7c7c7c' }}>{username}</Link> / <strong>{repo.name}</strong>
        </div>
        <h1>{repo.name}</h1>
        {repo.description && <p className="desc">{repo.description}</p>}
        <div className="actions">
          <Link to={`/repo/${repo.id}/commits/${branch}`} className="btn-sm">Commits</Link>
          <Link to={`/repo/${repo.id}/files`} className="btn-sm">Files</Link>
          <Link to={`/repo/${repo.id}/pages`} className="btn-sm">Pages</Link>
          <Link to={`/repo/${repo.id}/app`} className="btn-sm">App</Link>
          <Link to={`/repo/${repo.id}/ssh`} className="btn-sm">SSH</Link>
          <Link to={`/repo/${repo.id}/settings`} className="btn-sm">Settings</Link>
          <span style={{ fontSize: 12, color: '#7c7c7c', padding: '6px 0' }}>
            {repo.is_private ? 'Private' : 'Public'}
          </span>
        </div>
      </div>

      <div className="branch-bar">branch: {branch}</div>

      <div className="file-list">
        {entries.length === 0 ? (
          <div className="empty-state">
            <p>Empty repository</p>
            <div className="clone-url">git clone http://localhost:8080/git/{username}/{repo.name}</div>
          </div>
        ) : (
          entries.map(e => (
            <Link
              key={e.name}
              to={`/repo/${repo.id}/${e.is_dir ? 'tree' : 'blob'}/${branch}/${e.name}`}
              className="file-entry"
            >
              <span className="icon">{e.is_dir ? '📁' : '📄'}</span>
              <span>{e.name}</span>
            </Link>
          ))
        )}
      </div>

      {readmeHtml && <MarkdownView html={readmeHtml} />}

      {commits.length > 0 && (
        <div style={{ marginTop: 16 }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8, color: '#7c7c7c' }}>
            Recent commits
          </h3>
          <div className="commit-list">
            {commits.slice(0, 5).map(c => (
              <div key={c.sha} className="commit-entry">
                <div className="sha">{c.sha}</div>
                <div className="msg">{c.message.split('\n')[0]}</div>
        <div className="meta">
                <span style={{ fontSize: 12, color: '#7c7c7c', padding: '6px 0' }}>
                  {c.author}
                </span>
                <span style={{ fontSize: 12, color: '#7c7c7c', padding: '6px 0' }}>
                  {c.time}
                </span>
              </div>
            </div>
          ))}
          </div>
        </div>
      )}

      <div className="clone-url-bar" style={{ marginTop: 16, padding: '8px 12px', background: '#1a1a1a', borderRadius: 6, fontSize: 13, color: '#7c7c7c' }}>
        <strong>Clone:</strong> git clone http://{window.location.host}/git/{username}/{repo.name}
      </div>
    </div>
  )
}
