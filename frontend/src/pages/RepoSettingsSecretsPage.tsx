import { useEffect, useState, type FormEvent } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, listSecrets, createSecret, deleteSecret } from '../api'
import type { Repo, RepoSecret } from '../api'
import Spinner from '../components/Spinner'

export default function RepoSettingsSecretsPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [secrets, setSecrets] = useState<RepoSecret[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [name, setName] = useState('')
  const [value, setValue] = useState('')
  const [creating, setCreating] = useState(false)

  const load = async () => {
    if (!id) return
    setLoading(true)
    setErr('')
    try {
      const rid = parseInt(id)
      const [r, s] = await Promise.all([getRepo(rid), listSecrets(rid)])
      setRepo(r.repo)
      setSecrets(s.secrets)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '載入失敗')
    }
    setLoading(false)
  }

  useEffect(() => { load() }, [id])

  const handleCreate = async (e: FormEvent) => {
    e.preventDefault()
    if (!id || !name.trim() || !value.trim()) return
    setCreating(true)
    setErr('')
    try {
      await createSecret(parseInt(id), name.trim(), value)
      setName('')
      setValue('')
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '建立失敗')
    }
    setCreating(false)
  }

  const handleDelete = async (secretId: number) => {
    if (!id) return
    try {
      await deleteSecret(parseInt(id), secretId)
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '刪除失敗')
    }
  }

  if (loading) return <Spinner />
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">找不到</div>

  return (
    <div className="settings-page" style={{ maxWidth: 600 }}>
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / Secrets
      </div>
      <h2>Secrets</h2>

      <form onSubmit={handleCreate}>
        <label>名稱</label>
        <input type="text" value={name} onChange={e => setName(e.target.value)}
          placeholder="例如：API_KEY" />
        <label>值</label>
        <input type="password" value={value} onChange={e => setValue(e.target.value)}
          placeholder="秘密值" />
        <button type="submit" disabled={creating || !name.trim() || !value.trim()}>
          {creating ? '建立中...' : '新增'}
        </button>
      </form>

      {err && <p className="msg-err">{err}</p>}

      <hr />
      <h3>已有 Secrets</h3>
      {secrets.length === 0 && <p style={{ fontSize: 13, color: '#666' }}>尚無任何 Secret</p>}
      {secrets.map(s => (
        <div key={s.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0', borderBottom: '1px solid #eee' }}>
          <div>
            <strong>{s.name}</strong>
          </div>
          <button className="btn-sm danger" onClick={() => handleDelete(s.id)}>刪除</button>
        </div>
      ))}
    </div>
  )
}
