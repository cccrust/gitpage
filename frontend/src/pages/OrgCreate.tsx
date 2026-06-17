import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { createOrg } from '../api'

export default function OrgCreate() {
  const nav = useNavigate()
  const [name, setName] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [description, setDescription] = useState('')
  const [err, setErr] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) { setErr('組織名稱不能為空'); return }
    setSubmitting(true)
    setErr('')
    try {
      const res = await createOrg(name.trim(), displayName.trim() || name.trim(), description.trim())
      nav(`/org/${res.org.name}`)
    } catch (e: any) {
      setErr(e.message)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="page">
      <h2>建立組織</h2>
      <form onSubmit={handleSubmit} className="form">
        {err && <div className="error-box">{err}</div>}
        <label>
          組織名稱
          <input value={name} onChange={e => setName(e.target.value)} placeholder="my-org" required />
        </label>
        <label>
          顯示名稱
          <input value={displayName} onChange={e => setDisplayName(e.target.value)} placeholder="My Organization" />
        </label>
        <label>
          描述
          <textarea value={description} onChange={e => setDescription(e.target.value)} placeholder="組織描述" rows={3} />
        </label>
        <button type="submit" className="btn" disabled={submitting}>
          {submitting ? '建立中...' : '建立'}
        </button>
      </form>
    </div>
  )
}
