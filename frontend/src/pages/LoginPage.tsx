import { useState, FormEvent } from 'react'
import { useNavigate, Link } from 'react-router-dom'
import { login, setToken } from '../api'

export default function LoginPage() {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [err, setErr] = useState('')
  const nav = useNavigate()

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    setErr('')
    try {
      const res = await login(username, password)
      setToken(res.token)
      nav('/')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Login failed')
    }
  }

  return (
    <div className="auth-page">
      <h2>Login</h2>
      <form onSubmit={submit}>
        <label>Username</label>
        <input type="text" value={username} onChange={e => setUsername(e.target.value)} required />
        <label>Password</label>
        <input type="password" value={password} onChange={e => setPassword(e.target.value)} required />
        {err && <p className="msg-err">{err}</p>}
        <button className="btn" type="submit">Login</button>
      </form>
      <p className="switch">
        Don't have an account? <Link to="/register" style={{ textDecoration: 'underline' }}>Register</Link>
      </p>
    </div>
  )
}
