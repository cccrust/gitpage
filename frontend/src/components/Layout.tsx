import { Link, useLocation, useNavigate } from 'react-router-dom'
import { isLoggedIn, clearToken } from '../api'

export default function Layout({ children }: { children: React.ReactNode }) {
  const loc = useLocation()
  const nav = useNavigate()
  const loggedIn = isLoggedIn()
  const path = loc.pathname

  const doLogout = () => {
    clearToken()
    nav('/login')
  }

  return (
    <>
      <nav className="topnav">
        <div className="inner">
          <Link to="/" className="logo">gitpage</Link>
          <div className="spacer" />
          {loggedIn ? (
            <>
              <Link to="/new" className="nav-link">+ New</Link>
              <a href="#" className="nav-link" onClick={doLogout}>Logout</a>
            </>
          ) : (
            <>
              <Link to="/login" className="nav-link">Login</Link>
              <Link to="/register" className="nav-link">Register</Link>
            </>
          )}
        </div>
      </nav>

      <div className="main-content">
        <div className="container">
          {children}
        </div>
      </div>

      <nav className="bottom-nav">
        <Link to="/" className={path === '/' ? 'active' : ''}>Home</Link>
        <Link to="/new" className={path === '/new' ? 'active' : ''}>New</Link>
        {loggedIn ? (
          <a href="#" onClick={doLogout}>Logout</a>
        ) : (
          <Link to="/login" className={path.startsWith('/login') ? 'active' : ''}>Login</Link>
        )}
      </nav>
    </>
  )
}
