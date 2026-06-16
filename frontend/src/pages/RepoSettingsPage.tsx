import { useEffect, useState, FormEvent } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { getRepo, deleteRepo, isLoggedIn, clearToken } from '../api'
import type { Repo } from '../api'

export default function RepoSettingsPage() {
  const { id } = useParams<{ id: string }>()
  const nav = useNavigate()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [desc, setDesc] = useState('')
  const [isPrivate, setIsPrivate] = useState(false)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [confirmDelete, setConfirmDelete] = useState('')
  const [msg, setMsg] = useState('')
  const [err, setErr] = useState('')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }

    getRepo(numId)
      .then(r => {
        setRepo(r.repo)
        setDesc(r.repo.description)
        setIsPrivate(r.repo.is_private)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id])

  const save = async (e: FormEvent) => {
    e.preventDefault()
    if (!id) return
    setSaving(true)
    setMsg('')
    setErr('')
    try {
      const res = await fetch(`/api/repos/${id}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${localStorage.getItem('token')}`,
        },
        body: JSON.stringify({ description: desc, is_private: isPrivate }),
      })
      const d = await res.json()
      if (!res.ok) throw new Error(d.error || '儲存失敗')
      setMsg('Saved')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '儲存失敗')
    }
    setSaving(false)
  }

  const handleDelete = async () => {
    if (!id || confirmDelete !== repo?.name) return
    setDeleting(true)
    setErr('')
    try {
      await deleteRepo(parseInt(id))
      nav('/')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : '刪除失敗')
    }
    setDeleting(false)
  }

  if (loading) return <div className="loading">Loading...</div>
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">找不到</div>

  return (
    <div className="settings-page">
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / Settings
      </div>
      <h2>Repository Settings</h2>

      <form onSubmit={save}>
        <label>Description</label>
        <input type="text" value={desc} onChange={e => setDesc(e.target.value)} />

        <label className="checkbox">
          <input type="checkbox" checked={isPrivate} onChange={e => setIsPrivate(e.target.checked)} />
          Private repository
        </label>

        {msg && <p className="msg-ok">{msg}</p>}
        {err && <p className="msg-err">{err}</p>}

        <button className="btn" type="submit" disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </form>

      <div className="danger-zone">
        <h3>Danger Zone</h3>
        <p>Type <strong>{repo.name}</strong> to confirm deletion.</p>
        <input type="text" value={confirmDelete} onChange={e => setConfirmDelete(e.target.value)} placeholder={repo.name} />
        <button
          className="btn-sm danger"
          onClick={handleDelete}
          disabled={deleting || confirmDelete !== repo.name}
        >
          {deleting ? 'Deleting...' : 'Delete this repository'}
        </button>
      </div>
    </div>
  )
}
