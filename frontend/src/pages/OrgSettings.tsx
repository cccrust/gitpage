import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import Spinner from '../components/Spinner'
import { getOrg, updateOrg, isLoggedIn } from '../api'

export default function OrgSettings() {
  const { name } = useParams<{ name: string }>()
  const nav = useNavigate()
  const [displayName, setDisplayName] = useState('')
  const [description, setDescription] = useState('')
  const [err, setErr] = useState('')
  const [success, setSuccess] = useState(false)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (!name) return
    getOrg(name)
      .then(r => {
        setDisplayName(r.org.display_name)
        setDescription(r.org.description)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [name])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name) return
    setSaving(true)
    setErr('')
    setSuccess(false)
    try {
      await updateOrg(name, { display_name: displayName, description })
      setSuccess(true)
    } catch (e: any) {
      setErr(e.message)
    } finally {
      setSaving(false)
    }
  }

  if (loading) return <Spinner />
  if (!isLoggedIn()) return <div className="error-box">需要登入</div>

  return (
    <div className="page">
      <h2>組織設定 - {name}</h2>
      <form onSubmit={handleSubmit} className="form">
        {err && <div className="error-box">{err}</div>}
        {success && <div className="success-box">已更新</div>}
        <label>
          組織名稱（不可修改）
          <input value={name || ''} disabled />
        </label>
        <label>
          顯示名稱
          <input value={displayName} onChange={e => setDisplayName(e.target.value)} />
        </label>
        <label>
          描述
          <textarea value={description} onChange={e => setDescription(e.target.value)} rows={3} />
        </label>
        <button type="submit" className="btn" disabled={saving}>
          {saving ? '儲存中...' : '儲存'}
        </button>
        <button type="button" className="btn" style={{ marginLeft: 8 }} onClick={() => nav(`/org/${name}`)}>
          取消
        </button>
      </form>
    </div>
  )
}
