import { useEffect, useState, type FormEvent } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, listCollaborators, addCollaborator, removeCollaborator } from '../api'
import type { Repo, RepoCollaborator } from '../api'
import Spinner from '../components/Spinner'

export default function RepoSettingsCollaboratorsPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [collabs, setCollabs] = useState<RepoCollaborator[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [username, setUsername] = useState('')
  const [adding, setAdding] = useState(false)

  const load = async () => {
    if (!id) return
    setLoading(true)
    setErr('')
    try {
      const rid = parseInt(id)
      const [r, c] = await Promise.all([getRepo(rid), listCollaborators(rid)])
      setRepo(r.repo)
      setCollabs(c.collaborators)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '載入失敗')
    }
    setLoading(false)
  }

  useEffect(() => { load() }, [id])

  const handleAdd = async (e: FormEvent) => {
    e.preventDefault()
    if (!id || !username.trim()) return
    setAdding(true)
    setErr('')
    try {
      await addCollaborator(parseInt(id), username.trim())
      setUsername('')
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '新增失敗')
    }
    setAdding(false)
  }

  const handleRemove = async (userId: number) => {
    if (!id) return
    try {
      await removeCollaborator(parseInt(id), userId)
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '移除失敗')
    }
  }

  if (loading) return <Spinner />
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">找不到</div>

  return (
    <div className="settings-page" style={{ maxWidth: 600 }}>
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / Collaborators
      </div>
      <h2>協作者管理</h2>

      <form onSubmit={handleAdd}>
        <label>使用者名稱</label>
        <div style={{ display: 'flex', gap: 8 }}>
          <input type="text" value={username} onChange={e => setUsername(e.target.value)}
            placeholder="輸入使用者名稱" style={{ flex: 1 }} />
          <button type="submit" disabled={adding || !username.trim()}>
            {adding ? '新增中...' : '新增'}
          </button>
        </div>
      </form>

      {err && <p className="msg-err">{err}</p>}

      <hr />
      <h3>協作者列表</h3>
      {collabs.length === 0 && <p style={{ fontSize: 13, color: '#666' }}>尚無協作者</p>}
      {collabs.map(c => (
        <div key={c.user_id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0', borderBottom: '1px solid #eee' }}>
          <div>
            <strong>{c.username}</strong>
            <span style={{ fontSize: 12, color: '#888', marginLeft: 8 }}>{c.permission}</span>
          </div>
          <button className="btn-sm danger" onClick={() => handleRemove(c.user_id)}>移除</button>
        </div>
      ))}
    </div>
  )
}
