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
    if (isNaN(numId)) { setErr('Invalid ID'); setLoading(false); return }

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
      if (!res.ok) throw new Error(d.error || 'Save failed')
      setMsg('Saved')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Save failed')
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
      setErr(e instanceof Error ? e.message : 'Delete failed')
    }
    setDeleting(false)
  }

  if (loading) return <div className="loading">Loading...</div>
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">Not found</div>

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

        {msg && <p style={{ color: '#090', fontSize: 13, marginBottom: 8 }}>{msg}</p>}
        {err && <p style={{ color: '#c00', fontSize: 13, marginBottom: 8 }}>{err}</p>}

        <button className="btn" type="submit" disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </form>

      <hr style={{ margin: '24px 0', border: 'none', borderTop: '1px solid #333' }} />

      <h3 style={{ fontSize: 14, fontWeight: 600, color: '#c00', marginBottom: 8 }}>Danger Zone</h3>
      <p style={{ fontSize: 13, color: '#7c7c7c', marginBottom: 8 }}>
        Type <strong>{repo.name}</strong> to confirm deletion.
      </p>
      <input
        type="text"
        value={confirmDelete}
        onChange={e => setConfirmDelete(e.target.value)}
        placeholder={repo.name}
        style={{ marginBottom: 8 }}
      />
      <button
        className="btn-sm"
        style={{ background: '#c00', color: '#fff' }}
        onClick={handleDelete}
        disabled={deleting || confirmDelete !== repo.name}
      >
        {deleting ? 'Deleting...' : 'Delete this repository'}
      </button>
    </div>
  )
}
