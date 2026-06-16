import { useEffect, useState } from 'react'
import { me, updateProfile, changePassword, isLoggedIn } from '../api'
import type { User } from '../api'

export default function UserSettingsPage() {
  const [user, setUser] = useState<User | null>(null)
  const [bio, setBio] = useState('')
  const [avatarUrl, setAvatarUrl] = useState('')
  const [saving, setSaving] = useState(false)
  const [msg, setMsg] = useState('')

  const [curPw, setCurPw] = useState('')
  const [newPw, setNewPw] = useState('')
  const [changing, setChanging] = useState(false)
  const [pwMsg, setPwMsg] = useState('')

  useEffect(() => {
    me().then(d => {
      setUser(d.user)
      setBio(d.user.bio)
      setAvatarUrl(d.user.avatar_url)
    }).catch(() => {})
  }, [])

  const doSave = async () => {
    if (!user) return
    setSaving(true)
    setMsg('')
    try {
      await updateProfile(user.username, bio, avatarUrl)
      setMsg('儲存成功')
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : '儲存失敗')
    } finally {
      setSaving(false)
    }
  }

  const doChangePw = async () => {
    if (newPw.length < 6) { setPwMsg('新密碼至少需要 6 個字元'); return }
    setChanging(true)
    setPwMsg('')
    try {
      await changePassword(curPw, newPw)
      setPwMsg('密碼修改成功')
      setCurPw('')
      setNewPw('')
    } catch (e: unknown) {
      setPwMsg(e instanceof Error ? e.message : '修改密碼失敗')
    } finally {
      setChanging(false)
    }
  }

  if (!isLoggedIn()) return <div className="error-box">請先登入</div>
  if (!user) return <div className="loading">Loading...</div>

  return (
    <div className="profile-page" style={{ maxWidth: 500 }}>
      <h2>設定</h2>

      <section style={{ marginBottom: 32 }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>個人資料</h3>
        <label style={{ display: 'block', fontSize: 12, fontWeight: 600, marginBottom: 4 }}>Bio</label>
        <textarea value={bio} onChange={e => setBio(e.target.value)} rows={3}
          style={{ width: '100%', resize: 'vertical' }} />
        <label style={{ display: 'block', fontSize: 12, fontWeight: 600, margin: '12px 0 4px' }}>Avatar URL</label>
        <input value={avatarUrl} onChange={e => setAvatarUrl(e.target.value)} style={{ width: '100%' }} />
        <button onClick={doSave} disabled={saving} style={{ marginTop: 8 }}>
          {saving ? '儲存中...' : '儲存'}
        </button>
        {msg && <p style={{ fontSize: 12, marginTop: 4 }}>{msg}</p>}
      </section>

      <section>
        <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>修改密碼</h3>
        <label style={{ display: 'block', fontSize: 12, fontWeight: 600, marginBottom: 4 }}>目前密碼</label>
        <input type="password" value={curPw} onChange={e => setCurPw(e.target.value)} style={{ width: '100%' }} />
        <label style={{ display: 'block', fontSize: 12, fontWeight: 600, margin: '12px 0 4px' }}>新密碼</label>
        <input type="password" value={newPw} onChange={e => setNewPw(e.target.value)} style={{ width: '100%' }} />
        <button onClick={doChangePw} disabled={changing} style={{ marginTop: 8 }}>
          {changing ? '修改中...' : '修改密碼'}
        </button>
        {pwMsg && <p style={{ fontSize: 12, marginTop: 4 }}>{pwMsg}</p>}
      </section>
    </div>
  )
}