import { useState, FormEvent } from 'react'
import { useNavigate, Link } from 'react-router-dom'
import { register, setToken } from '../api'

export default function RegisterPage() {
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [err, setErr] = useState('')
  const nav = useNavigate()

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    setErr('')
    try {
      const res = await register(username, email, password)
      setToken(res.token)
      nav('/')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Registration failed')
    }
  }

  return (
    <div className="auth-page">
      <h2>Register</h2>
      <form onSubmit={submit}>
        <label>Username</label>
        <input type="text" value={username} onChange={e => setUsername(e.target.value)} required />
        <label>Email</label>
        <input type="email" value={email} onChange={e => setEmail(e.target.value)} required />
        <label>Password</label>
        <input type="password" value={password} onChange={e => setPassword(e.target.value)} required />
        {err && <p className="msg-err">{err}</p>}
        <button className="btn" type="submit">Register</button>
      </form>
      <p className="switch">
        Already have an account? <Link to="/login" style={{ textDecoration: 'underline' }}>Login</Link>
      </p>
    </div>
  )
}
