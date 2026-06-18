import { useEffect, useState, type FormEvent } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, listBranchProtections, createBranchProtection, deleteBranchProtection } from '../api'
import type { Repo, BranchProtection } from '../api'
import Spinner from '../components/Spinner'

export default function RepoSettingsBranchProtectionPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [protections, setProtections] = useState<BranchProtection[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [pattern, setPattern] = useState('')
  const [creating, setCreating] = useState(false)

  const load = async () => {
    if (!id) return
    setLoading(true)
    setErr('')
    try {
      const rid = parseInt(id)
      const [r, p] = await Promise.all([getRepo(rid), listBranchProtections(rid)])
      setRepo(r.repo)
      setProtections(p.branch_protections)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '載入失敗')
    }
    setLoading(false)
  }

  useEffect(() => { load() }, [id])

  const handleCreate = async (e: FormEvent) => {
    e.preventDefault()
    if (!id || !pattern.trim()) return
    setCreating(true)
    setErr('')
    try {
      await createBranchProtection(parseInt(id), pattern.trim())
      setPattern('')
      await load()
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '建立失敗')
    }
    setCreating(false)
  }

  const handleDelete = async (protectionId: number) => {
    if (!id) return
    try {
      await deleteBranchProtection(parseInt(id), protectionId)
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
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / Branch Protection
      </div>
      <h2>Branch Protection</h2>

      <form onSubmit={handleCreate}>
        <label>分支模式</label>
        <div style={{ display: 'flex', gap: 8 }}>
          <input type="text" value={pattern} onChange={e => setPattern(e.target.value)}
            placeholder="例如：main、release/*" style={{ flex: 1 }} />
          <button type="submit" disabled={creating || !pattern.trim()}>
            {creating ? '建立中...' : '新增'}
          </button>
        </div>
      </form>

      {err && <p className="msg-err">{err}</p>}

      <hr />
      <h3>保護規則</h3>
      {protections.length === 0 && <p style={{ fontSize: 13, color: '#666' }}>尚無任何保護規則</p>}
      {protections.map(p => (
        <div key={p.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0', borderBottom: '1px solid #eee' }}>
          <div>
            <strong>{p.pattern}</strong>
            <span style={{ fontSize: 12, color: '#888', marginLeft: 8 }}>
              PR: {p.require_pr ? '是' : '否'} / 審查: {p.require_approvals} / 過時審查: {p.dismiss_stale_reviews ? '是' : '否'}
            </span>
          </div>
          <button className="btn-sm danger" onClick={() => handleDelete(p.id)}>刪除</button>
        </div>
      ))}
    </div>
  )
}
