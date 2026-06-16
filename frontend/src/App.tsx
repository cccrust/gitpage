import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import Layout from './components/Layout'
import LoginPage from './pages/LoginPage'
import RegisterPage from './pages/RegisterPage'
import Dashboard from './pages/Dashboard'
import NewRepoPage from './pages/NewRepoPage'
import RepoPage from './pages/RepoPage'
import FileViewPage from './pages/FileViewPage'
import CommitsPage from './pages/CommitsPage'

export default function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/register" element={<RegisterPage />} />
          <Route path="/new" element={<NewRepoPage />} />
          <Route path="/repo/:id" element={<RepoPage />} />
          <Route path="/repo/:id/*" element={<FileViewPage />} />
          <Route path="/repo/:id/commits/:branch" element={<CommitsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  )
}
